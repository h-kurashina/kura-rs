# kura-rs

**Copy the code, keep the proof.**

kura-rs is a collection of small Rust parts for AI data processing (chunking, text normalization, deduplication, token counting) and security (log parsing, pattern matching, hashing, binary analysis).

- **Rust users** install the CLI with `cargo install kura-rs` and run `kura add <name>` to copy a part's source into their own project, like shadcn/ui.
- **Python users** install the PyO3 build: `pip install kura-rs`.

Every part ships with evidence: differential tests against a reference implementation and benchmarks across input sizes.

> `minhash` and `file-hash` are measured: their verification and benchmark numbers are written by `verify/run.py` after actually running the differential tests and benchmarks. The other parts in the registry still use **sample data** (`"sample": true`): their numbers are placeholders, not measurements.

## Repository layout

| Path | What it is |
| --- | --- |
| `crates/kura-rs/` | The CLI published to crates.io as `kura-rs`. Installs the `kura` command (and `kura-rs`, for when another tool already owns `kura`): `kura add <name>`, `kura list`. See [its README](crates/kura-rs/README.md). |
| `crates/kura-schema/` | The schema as Rust structs (serde + schemars + ts-rs). The single source of truth for the data shape. |
| `crates/kura-registry/` | CLI that validates `registry/*.json` and writes the site's JSON. |
| `crates/kura-server/` | axum registry API. Serves the same JSON as the static site. |
| `registry/*.json` | One file per part. The single source of truth for content. |
| `schema/` | Generated JSON Schema (committed). |
| `site/` | Next.js documentation and registry site (static export). |
| `site/src/generated/` | Generated TypeScript types (committed). |

## Commands

```sh
# Validate registry/*.json
cargo run -p kura-registry -- check

# Validate and write site/public/r/*.json, index.json and site/public/schema/
cargo run -p kura-registry -- build

# Regenerate schema/*.json and site/src/generated/*.ts after changing kura-schema
cargo run -p kura-registry -- codegen
cargo run -p kura-registry -- codegen --check   # fail if out of date

# Try the CLI against this checkout (no network)
cargo run -p kura-rs --bin kura -- add minhash --registry . --dry-run
cargo run -p kura-rs --bin kura -- list --registry .

# Site (runs the registry build first)
cd site && npm install && npm run dev
cd site && npm run build   # static export to site/out/
```

## Adding a part

Add `registry/<name>.json` (see `schema/part.schema.json`). The sidebar, the parts list, the detail page at `/parts/<name>` and the machine endpoint at `/r/<name>.json` are all generated from it.

## Languages

English is served at `/`. Japanese, Simplified Chinese, Korean, Spanish, French, German and Brazilian Portuguese are served at `/ja/`, `/zh/`, `/ko/`, `/es/`, `/fr/`, `/de/` and `/pt/`.

To add a language:

1. Copy `site/src/i18n/locales/en.ts` to `site/src/i18n/locales/<code>.ts` and translate it. Each dictionary is type-checked against the English one, so a missing key fails the build.
2. Register it in `LOCALE_INFO` in `site/src/i18n/dictionaries.ts`.
3. Add a `<code>` field to `Translations` in `crates/kura-schema/src/part.rs` and run `cargo run -p kura-registry -- codegen`. The site fails to type-check until the schema knows the language.

Part titles and descriptions are translated in the registry with `translations.<code>.title` / `translations.<code>.description`; omitted fields fall back to English.

## Machine-readable endpoints

- `/r/index.json` — all parts
- `/r/<name>.json` — one part, identical to the validated registry entry
- `/schema/part.schema.json`, `/schema/index.schema.json`

## Deploying to Vercel

The site is a static export, so no server is needed.

1. Import the repository in Vercel and set **Root Directory** to `site`.
2. That's it. `site/vercel.json` installs Rust (pinned in `rust-toolchain.toml`) before `npm ci`, so the registry is validated and written during the build.

The canonical URL, sitemap and OGP images use the production domain Vercel provides. Set `NEXT_PUBLIC_SITE_URL` to override it (e.g. for a custom domain).

## Registry API server

```sh
cargo run -p kura-server                      # listens on 127.0.0.1:8080
KURA_ADDR=0.0.0.0:8080 KURA_REGISTRY_DIR=./registry cargo run -p kura-server
```

| Route | Response |
| --- | --- |
| `GET /health` | `{"status":"ok","parts":1}` |
| `GET /r/index.json` | All parts (same bytes as the static site) |
| `GET /r/{name}.json` | One part, or `404` |
| `GET /api/parts?shelf=ai&q=hash` | Parts filtered by shelf and a case-insensitive text match |

Log level is controlled by `RUST_LOG` (default `kura_server=info,tower_http=info`).

## Checks

```sh
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
cargo run -p kura-registry -- codegen --check
cd site && npm run build
```

## Contributing

Every pull request runs [CI](.github/workflows/ci.yml): formatting, clippy, tests, registry validation, a check that generated files are up to date, and a full site build. Parts without a translated description are listed as notices on the pull request.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
