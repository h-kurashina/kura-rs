# kura-rs

Copy small, verified Rust parts into your project, like shadcn/ui.

```sh
cargo install kura-rs
cd my-project
kura add minhash
```

`kura add minhash` copies the part's source into `src/parts/minhash/`, runs `cargo add` for its dependencies and tells you how to declare the module:

```text
Adding MinHash (minhash 0.1.0) from https://raw.githubusercontent.com/h-kurashina/kura-rs/main/
  create    src/parts/minhash/mod.rs
  create    src/parts/minhash/permutation.rs

Dependencies (Cargo.toml):
  cargo add sha1@0.11

Copied `minhash` into src/parts/minhash

Next steps:
  1. Add `pub mod minhash;` to src/parts/mod.rs
  2. Add `mod parts;` to src/main.rs or src/lib.rs
  3. Use it: ...
```

The code is yours from then on: edit it freely. Every part in the registry ships with differential tests against a reference implementation and benchmarks; see <https://github.com/h-kurashina/kura-rs>.

If another tool already installs a `kura` command, install only the `kura-rs` binary:

```sh
cargo install kura-rs --bin kura-rs
kura-rs add minhash
```

## Commands

```sh
kura list                       # parts in the registry
kura add <name>                 # copy into src/parts/<name>/ and run `cargo add`
kura add <name> --dry-run       # show what would happen, write nothing
kura add <name> --dir src/x     # copy somewhere else
kura add <name> --overwrite     # replace files you changed
kura add <name> --no-deps       # print the dependency lines instead of running `cargo add`
```

- Existing files are never replaced without `--overwrite`. Identical files are left as they are, so running `add` again is safe.
- Without a `Cargo.toml` (with `[package]`) in the current directory or above, the dependency lines are printed instead.
- Part names with `-` become module directories with `_` (`file-hash` → `src/parts/file_hash/`).

## Where parts come from

By default, parts are read from the `main` branch of the GitHub repository (`https://raw.githubusercontent.com/h-kurashina/kura-rs/main/`). Point `--registry` or the `KURA_REGISTRY` environment variable at another source:

```sh
kura add minhash --registry ./kura                 # a local clone (works offline)
KURA_REGISTRY=https://example.com/kura/ kura list  # a mirror with the same layout
```

A source has the repository's layout: `registry/<name>.json` and the files it lists (e.g. `parts/minhash/mod.rs`).

`kura list` reads `registry/*.json` from a local source. For the default GitHub source it lists `registry/` through the GitHub contents API (`api.github.com`, 60 requests per hour without a token). Other URLs must serve `registry/index.json` (the output of `kura-registry build`, same as the site's `/r/index.json`).

## Safety

Registry data is treated as untrusted input:

- Part names must be kebab-case.
- File paths must be relative, made of ASCII letters, digits, `_`, `-` and `.`, with no `..`, `.`, empty segments, backslashes, drive letters, hidden files or Windows device names.
- Crate names, versions and features passed to `cargo add` are checked so they cannot be read as options.
- All files are fetched and checked before anything is written; a missing or non-UTF-8 file aborts without writing.
- Files larger than 4 MiB are refused. Symbolic links at the destination are never written through.
- Text from the registry is printed without control characters.

## License

MIT OR Apache-2.0
