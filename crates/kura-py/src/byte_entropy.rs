//! `kura_rs.byte_entropy`：parts/byte_entropy を Python から使うための薄い包み。
//! 計算は Rust 側に任せ、ここでは Python の値との変換と、誤った使い方のエラーだけを扱う。
//! 計算している間は GIL を手放し、ほかのスレッドが動けるようにする。

use kura_parts::byte_entropy::{self, WindowError};
use pyo3::exceptions::{PyBufferError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyMemoryView, PyString};

/// これより短い入力は GIL を持ったまま計算する（手放す手間のほうが大きい）
const DETACH_THRESHOLD: usize = 2048;

/// bytes / bytearray / memoryview（そのほか連続したバッファ）の中身を `f` に渡し、GIL を手放して計算する。
///
/// bytes は書き換えられないので写さずに読む。bytearray などの書き換えられるバッファは、
/// 計算中にほかのスレッドが書き換えても答えが崩れないよう、GIL を持っている間に bytes に写してから計算する。
fn with_bytes<T: Send>(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    f: impl FnOnce(&[u8]) -> T + Send,
) -> PyResult<T> {
    let copied;
    let bytes = if let Ok(bytes) = data.cast::<PyBytes>() {
        bytes
    } else {
        if data.is_instance_of::<PyString>() {
            return Err(PyTypeError::new_err(
                "Strings must be encoded first (e.g. text.encode())",
            ));
        }
        let view = PyMemoryView::from(data).map_err(|_| {
            PyTypeError::new_err(format!(
                "a bytes-like object is required, not '{}'",
                data.get_type()
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ))
        })?;
        if !view.getattr("c_contiguous")?.extract::<bool>()? {
            // 飛び飛びのバッファ（memoryview(b)[::2] など）は hashlib と同じく受け付けない
            return Err(PyBufferError::new_err(
                "memoryview: underlying buffer is not C-contiguous",
            ));
        }
        // 中身をバイト単位の平らな並びとして写す（NumPy の配列なら、要素のバイト列そのもの）
        copied = view.call_method0("tobytes")?.cast_into::<PyBytes>()?;
        &copied
    };
    let slice = bytes.as_bytes();
    if slice.len() < DETACH_THRESHOLD {
        Ok(f(slice))
    } else {
        Ok(py.detach(|| f(slice)))
    }
}

/// 窓の大きさ・刻みを確かめる（0 以下は ValueError。Python の負の数もここで弾く）
fn positive(name: &str, value: i64) -> PyResult<usize> {
    if value < 1 {
        return Err(PyValueError::new_err(format!(
            "{name} must be at least 1 byte, got {value}"
        )));
    }
    usize::try_from(value).map_err(|_| PyValueError::new_err(format!("{name} is too large")))
}

fn window_error(e: WindowError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Shannon entropy of the bytes in bits per byte, from 0.0 to 8.0.
///
/// Bit-for-bit the same float as
/// `scipy.stats.entropy(numpy.bincount(numpy.frombuffer(data, numpy.uint8), minlength=256), base=2)`,
/// except that empty data gives 0.0 (SciPy gives nan).
#[pyfunction]
fn entropy(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<f64> {
    with_bytes(py, data, byte_entropy::entropy)
}

/// How often each byte value 0..255 occurs (a list of 256 ints).
#[pyfunction]
fn histogram(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Vec<u64>> {
    with_bytes(py, data, |d| byte_entropy::histogram(d).to_vec())
}

/// Entropy in bits of 256 counts (e.g. added-up `histogram` results of pieces of a file),
/// the same as `scipy.stats.entropy(counts, base=2)`. All zero gives 0.0.
#[pyfunction]
fn entropy_of_histogram(counts: Vec<u64>) -> PyResult<f64> {
    let counts: byte_entropy::Histogram = counts.try_into().map_err(|c: Vec<u64>| {
        PyValueError::new_err(format!("expected 256 counts, got {}", c.len()))
    })?;
    Ok(byte_entropy::entropy_of_histogram(&counts))
}

/// Entropy of each `window`-byte window starting at offsets 0, step, 2 * step, ...
/// (`step` defaults to `window`: windows side by side). Only windows that fit entirely
/// in the data are included, so data shorter than `window` gives an empty list.
///
/// Returns a list of `(offset, entropy)` tuples.
#[pyfunction]
#[pyo3(signature = (data, window, step = None))]
fn windows(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    window: i64,
    step: Option<i64>,
) -> PyResult<Vec<(usize, f64)>> {
    let window = positive("window", window)?;
    let step = positive("step", step.unwrap_or(window as i64))?;
    with_bytes(py, data, |d| {
        byte_entropy::windows(d, window, step)
            .map(|it| it.map(|w| (w.offset, w.entropy)).collect::<Vec<_>>())
    })?
    .map_err(window_error)
}

/// Number of windows `windows` returns for data of `length` bytes:
/// `(length - window) // step + 1` when `length >= window`, otherwise 0.
#[pyfunction]
#[pyo3(signature = (length, window, step = None))]
fn window_count(length: u64, window: i64, step: Option<i64>) -> PyResult<usize> {
    let window = positive("window", window)?;
    let step = positive("step", step.unwrap_or(window as i64))?;
    let length =
        usize::try_from(length).map_err(|_| PyValueError::new_err("length is too large"))?;
    byte_entropy::window_count(length, window, step).map_err(window_error)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "byte_entropy")?;
    m.add_function(wrap_pyfunction!(entropy, &m)?)?;
    m.add_function(wrap_pyfunction!(histogram, &m)?)?;
    m.add_function(wrap_pyfunction!(entropy_of_histogram, &m)?)?;
    m.add_function(wrap_pyfunction!(windows, &m)?)?;
    m.add_function(wrap_pyfunction!(window_count, &m)?)?;
    m.add("MAX_ENTROPY", byte_entropy::MAX_ENTROPY)?;
    parent.add_submodule(&m)
}
