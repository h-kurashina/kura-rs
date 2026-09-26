// macOS で `cargo build` / `cargo clippy` を通すための設定（maturin を使う場合は不要だが、害もない）
fn main() {
    pyo3_build_config::add_extension_module_link_args();
}
