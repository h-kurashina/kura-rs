//! kura-rs の Python 向けの入口（`kura_rs._native`）。
//! 部品ごとの薄い包み（src/<部品>.rs）を、Python のサブモジュールとして登録する。
//! Python からは kura_rs/<部品>.py 経由で `from kura_rs import minhash` のように使う。

use pyo3::prelude::*;

mod byte_entropy;
mod file_hash;
mod minhash;
mod multi_pattern_match;
mod unicode_normalize;

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    minhash::register(m)?;
    file_hash::register(m)?;
    multi_pattern_match::register(m)?;
    byte_entropy::register(m)?;
    unicode_normalize::register(m)?;
    Ok(())
}
