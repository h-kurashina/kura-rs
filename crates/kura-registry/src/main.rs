use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use kura_registry::Paths;
use kura_schema::Part;

/// kura のレジストリを検証し、サイト用の JSON と生成物を書き出す。
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Repository root (defaults to the workspace this binary was built from).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate registry/*.json without writing anything.
    Check,
    /// Validate, then write site/public/r/*.json, index.json and site/public/schema/.
    Build,
    /// Regenerate schema/*.json and site/src/generated/*.ts from kura-schema.
    Codegen {
        /// Fail instead of writing if the generated files are out of date.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = cli
        .root
        .map(Paths::new)
        .unwrap_or_else(Paths::from_manifest);

    let result = match cli.command {
        Command::Check => kura_registry::check(&paths).map(|parts| {
            println!("registry ok: {} part(s)", parts.len());
            report_missing_translations(&parts);
        }),
        Command::Build => kura_registry::build(&paths).map(|parts| {
            let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
            println!("registry built: {} → site/public/r/", names.join(", "));
            report_missing_translations(&parts);
        }),
        Command::Codegen { check: true } => kura_registry::codegen_check(&paths).map(|()| {
            println!("generated files are up to date");
        }),
        Command::Codegen { check: false } => kura_registry::codegen(&paths).map(|()| {
            println!("wrote schema/*.json and site/src/generated/*.ts");
        }),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// 説明文の訳がない部品と言語を知らせる。失敗にはしない。
/// GitHub Actions の中では注釈（::notice::）として出し、プルリクエストの画面に表示させる。
fn report_missing_translations(parts: &[Part]) {
    let in_actions = std::env::var_os("GITHUB_ACTIONS").is_some();
    for part in parts {
        let missing = part.missing_translations();
        if missing.is_empty() {
            continue;
        }
        let message = format!(
            "{}: no translated description for {}",
            part.name,
            missing.join(", ")
        );
        if in_actions {
            println!(
                "::notice file=registry/{}.json,title=Missing translations::{message}",
                part.name
            );
        } else {
            println!("note: {message}");
        }
    }
}
