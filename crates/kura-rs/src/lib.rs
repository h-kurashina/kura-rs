//! kura-rs の CLI 本体。`kura` と `kura-rs` の2つのコマンド（src/bin/）から呼ばれる。
//!
//! - `kura add <name>` — 部品のソースを取得元から取ってきて、自分のプロジェクトにコピーする
//! - `kura list` — 取得元にある部品の一覧
//!
//! 公開しているモジュールはテストのためのもので、安定した API ではない。

#![forbid(unsafe_code)]

pub mod manifest;
pub mod paths;
pub mod source;

use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};

use crate::manifest::{CheckedPart, Dependency, Manifest};
use crate::paths::{check_part_name, install_layout, module_name};
use crate::source::{FetchError, Listing, Source};

/// Copy small, verified Rust parts into your project.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Copy a part's source files into your project and add its dependencies.
    Add(AddArgs),
    /// List the parts available in the registry.
    List {
        #[command(flatten)]
        registry: RegistryArg,
    },
}

#[derive(clap::Args)]
struct RegistryArg {
    /// Where to fetch parts from: a URL or a local copy of the kura repository.
    /// Overrides the KURA_REGISTRY environment variable.
    /// [default: https://raw.githubusercontent.com/h-kurashina/kura-rs/main/]
    #[arg(long, value_name = "URL|DIR")]
    registry: Option<String>,
}

#[derive(clap::Args)]
struct AddArgs {
    /// Part name, e.g. `minhash`.
    name: String,
    /// Directory to write the files into. [default: src/parts/<name>/ next to Cargo.toml]
    #[arg(long, value_name = "PATH")]
    dir: Option<PathBuf>,
    /// Replace files that already exist with different content.
    #[arg(long)]
    overwrite: bool,
    /// Show what would happen without writing anything.
    #[arg(long)]
    dry_run: bool,
    /// Do not run `cargo add`; print the dependency lines instead.
    #[arg(long)]
    no_deps: bool,
    #[command(flatten)]
    registry: RegistryArg,
}

/// CLI の入口。
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Add(args) => add(args),
        Command::List { registry } => list(registry.registry.as_deref()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// 書き込み先 1 ファイルの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Create,
    Unchanged,
    Overwrite,
    Conflict,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Unchanged => "unchanged",
            Action::Overwrite => "overwrite",
            Action::Conflict => "conflict",
        }
    }
}

struct Planned {
    dest: PathBuf,
    contents: Vec<u8>,
    action: Action,
}

fn add(args: AddArgs) -> anyhow::Result<()> {
    check_part_name(&args.name).map_err(anyhow::Error::msg)?;
    let source = Source::resolve(args.registry.registry.as_deref()).map_err(anyhow::Error::msg)?;
    let cwd = std::env::current_dir().context("cannot read the current directory")?;
    let manifest_path = find_package_manifest(&cwd);

    // 1. 部品の JSON を取って確かめる
    let json = source.fetch_part_json(&args.name).map_err(|e| match e {
        FetchError::NotFound(_) => anyhow::anyhow!(
            "part `{}` not found in {source} (run `kura list` to see available parts)",
            args.name
        ),
        other => anyhow::anyhow!("cannot fetch part `{}`: {other}", args.name),
    })?;
    let part = Manifest::from_json(&json)
        .and_then(|m| {
            m.check(Some(&args.name))
                .map_err(|errs| errs.join("\n  - "))
        })
        .map_err(|e| anyhow::anyhow!("registry/{}.json is invalid:\n  - {e}", args.name))?;
    let CheckedPart { manifest, files } = part;

    // 2. 書き込み先を決め、全ファイルを先に取ってくる（途中で失敗しても何も書かないため）
    let module = module_name(&manifest.name);
    let dest_dir = match &args.dir {
        Some(dir) => cwd.join(dir),
        None => {
            let root = manifest_path
                .as_deref()
                .and_then(Path::parent)
                .unwrap_or(&cwd);
            root.join("src").join("parts").join(&module)
        }
    };
    let layout = install_layout(&files).map_err(anyhow::Error::msg)?;

    println!(
        "{} {} ({} {}) from {source}",
        if args.dry_run { "Would add" } else { "Adding" },
        clean(&manifest.title),
        manifest.name,
        clean(&manifest.version)
    );
    if manifest.sample {
        println!("note: this part is marked as sample data; its numbers are placeholders");
    }

    let mut plan = Vec::new();
    for (src, rel) in files.iter().zip(&layout) {
        let contents = source.fetch(src).map_err(|e| match e {
            FetchError::NotFound(_) => anyhow::anyhow!(
                "file {src} listed by part `{}` is missing from {source}",
                manifest.name
            ),
            other => anyhow::anyhow!("cannot fetch {src}: {other}"),
        })?;
        if std::str::from_utf8(&contents).is_err() {
            bail!("{src} is not valid UTF-8 text; refusing to write it");
        }
        let dest = rel.under(&dest_dir);
        let action = plan_action(&dest, &contents, args.overwrite)?;
        plan.push(Planned {
            dest,
            contents,
            action,
        });
    }

    // 3. 既存ファイルとぶつかるなら、何も書かずに止める
    let conflicts: Vec<&Planned> = plan
        .iter()
        .filter(|p| p.action == Action::Conflict)
        .collect();
    if !conflicts.is_empty() {
        let mut msg = String::from(
            "these files already exist with different content (use --overwrite to replace them):",
        );
        for c in &conflicts {
            msg.push_str(&format!("\n  {}", display_path(&c.dest, &cwd)));
        }
        bail!(msg);
    }

    for p in &plan {
        println!("  {:<9} {}", p.action.label(), display_path(&p.dest, &cwd));
        if !args.dry_run && matches!(p.action, Action::Create | Action::Overwrite) {
            write_file(&p.dest, &p.contents)?;
        }
    }

    // 4. 依存クレート
    let deps = &manifest.rust.dependencies;
    add_dependencies(deps, manifest_path.as_deref(), &args, &cwd)?;

    // 5. まとめと次の一歩
    println!();
    println!(
        "{} `{}` into {}",
        if args.dry_run {
            "Dry run: nothing was written. Would copy"
        } else {
            "Copied"
        },
        manifest.name,
        display_path(&dest_dir, &cwd)
    );
    print_next_steps(&manifest, &module, &dest_dir, args.dir.is_none(), &cwd);
    Ok(())
}

/// 書き込み先の今の状態から、どうするかを決める。
fn plan_action(dest: &Path, contents: &[u8], overwrite: bool) -> anyhow::Result<Action> {
    let meta = match std::fs::symlink_metadata(dest) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Action::Create),
        Err(e) => return Err(e).with_context(|| format!("cannot inspect {}", dest.display())),
    };
    if meta.file_type().is_symlink() {
        bail!(
            "{} is a symbolic link; refusing to write through it",
            dest.display()
        );
    }
    if !meta.is_file() {
        bail!("{} exists and is not a regular file", dest.display());
    }
    let existing =
        std::fs::read(dest).with_context(|| format!("cannot read {}", dest.display()))?;
    Ok(if existing == contents {
        Action::Unchanged
    } else if overwrite {
        Action::Overwrite
    } else {
        Action::Conflict
    })
}

fn write_file(dest: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::write(dest, contents).with_context(|| format!("cannot write {}", dest.display()))
}

/// `cargo add` を走らせるか、Cargo.toml に書く行を表示する。
fn add_dependencies(
    deps: &[Dependency],
    manifest_path: Option<&Path>,
    args: &AddArgs,
    cwd: &Path,
) -> anyhow::Result<()> {
    if deps.is_empty() {
        return Ok(());
    }
    let print_lines = |why: &str| {
        println!("\nAdd these dependencies to your Cargo.toml ({why}):");
        println!("  [dependencies]");
        for d in deps {
            println!("  {}", d.toml_line());
        }
    };
    let Some(manifest_path) = manifest_path.filter(|_| !args.no_deps) else {
        print_lines(if args.no_deps {
            "--no-deps"
        } else {
            "no Cargo.toml with [package] found"
        });
        return Ok(());
    };

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    println!("\nDependencies ({}):", display_path(manifest_path, cwd));
    for d in deps {
        let mut cmd_args: Vec<OsString> = vec!["add".into(), "--manifest-path".into()];
        cmd_args.push(manifest_path.as_os_str().to_owned());
        cmd_args.extend(d.cargo_add_args().into_iter().map(OsString::from));
        println!(
            "  {}cargo add {}",
            if args.dry_run { "would run: " } else { "" },
            d.cargo_add_args().join(" ")
        );
        if args.dry_run {
            continue;
        }
        let _ = std::io::stdout().flush();
        let status = Process::new(&cargo)
            .args(&cmd_args)
            .status()
            .with_context(|| format!("cannot run {}", cargo.to_string_lossy()))?;
        if !status.success() {
            print_lines("`cargo add` failed");
            bail!(
                "`cargo add {}` failed ({status}); the source files were copied, add the dependency manually",
                d.cargo_add_args().join(" ")
            );
        }
    }
    Ok(())
}

fn print_next_steps(
    manifest: &Manifest,
    module: &str,
    dest_dir: &Path,
    default_dir: bool,
    cwd: &Path,
) {
    println!("\nNext steps:");
    let mut step = 1;
    if default_dir {
        let parts_dir = dest_dir.parent().unwrap_or(dest_dir);
        let parts_mod = parts_dir.join("mod.rs");
        let src_dir = parts_dir.parent().unwrap_or(parts_dir);
        let declared =
            |file: &Path, decl: &str| std::fs::read_to_string(file).is_ok_and(|s| s.contains(decl));
        if !declared(&parts_mod, &format!("mod {module};")) {
            println!(
                "  {step}. Add `pub mod {module};` to {}",
                display_path(&parts_mod, cwd)
            );
            step += 1;
        }
        if !["main.rs", "lib.rs"]
            .iter()
            .any(|f| declared(&src_dir.join(f), "mod parts;"))
        {
            println!("  {step}. Add `mod parts;` to src/main.rs or src/lib.rs");
            step += 1;
        }
    } else {
        println!(
            "  {step}. Declare `mod {};` in the parent module of {}",
            dest_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| module.to_owned()),
            display_path(dest_dir, cwd)
        );
        step += 1;
    }
    match manifest.usage.as_ref().and_then(|u| u.rust.as_deref()) {
        Some(usage) if !usage.trim().is_empty() => {
            println!("  {step}. Use it:\n");
            for line in usage.lines() {
                println!("{}", format!("      {}", clean(line)).trim_end());
            }
        }
        _ => println!("  {step}. Use it via `crate::parts::{module}`"),
    }
}

/// 取得元から来た文字列を端末に出す前に、制御文字（エスケープシーケンスなど）を取り除く。
pub fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| !c.is_control())
        .collect()
}

/// 表示用に、今いるディレクトリからの相対パスにする。
fn display_path(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}

/// 今いるディレクトリから上へたどり、`[package]` を持つ Cargo.toml を探す。
/// ワークスペースのルート（`[package]` なし）は飛ばして上へ進む。
pub fn find_package_manifest(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        let path = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&path).ok()?;
        text.lines()
            .any(|l| l.trim() == "[package]")
            .then_some(path)
    })
}

fn list(registry: Option<&str>) -> anyhow::Result<()> {
    let source = Source::resolve(registry).map_err(anyhow::Error::msg)?;
    let listing = source::list_names(&source, source::GITHUB_API).map_err(anyhow::Error::msg)?;
    let mut summaries = match listing {
        Listing::Summaries(s) => s
            .into_iter()
            .filter(|s| {
                let ok = paths::is_kebab_case(&s.name);
                if !ok {
                    eprintln!("warning: skipping invalid part name {:?}", s.name);
                }
                ok
            })
            .collect(),
        Listing::Names(names) => {
            let mut out = Vec::new();
            for name in names {
                if check_part_name(&name).is_err() || name == "index" {
                    if name != "index" {
                        eprintln!("warning: skipping registry/{name}.json (invalid part name)");
                    }
                    continue;
                }
                let summary = source
                    .fetch_part_json(&name)
                    .map_err(|e| e.to_string())
                    .and_then(|json| Manifest::from_json(&json))
                    .and_then(|m| m.check(Some(&name)).map_err(|errs| errs.join("; ")));
                match summary {
                    Ok(part) => out.push(source::Summary {
                        name: part.manifest.name,
                        title: part.manifest.title,
                        shelves: part.manifest.shelves,
                        sample: part.manifest.sample,
                    }),
                    Err(e) => eprintln!("warning: skipping registry/{name}.json: {e}"),
                }
            }
            out
        }
    };
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    if summaries.is_empty() {
        bail!("no parts found in {source}");
    }

    let name_w = summaries
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let title_w = summaries
        .iter()
        .map(|s| s.title.chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    println!("{:<name_w$}  {:<title_w$}  SHELVES", "NAME", "TITLE");
    for s in &summaries {
        println!(
            "{:<name_w$}  {:<title_w$}  {}{}",
            s.name,
            clean(&s.title),
            clean(&s.shelves.join(",")),
            if s.sample { "  (sample)" } else { "" }
        );
    }
    println!(
        "\n{} part(s). Install one with `kura add <name>`.",
        summaries.len()
    );
    Ok(())
}
