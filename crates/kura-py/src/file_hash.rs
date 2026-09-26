//! `kura_rs.file_hash`：parts/file_hash を Python から使うための薄い包み。
//! 計算は Rust 側に任せ、ここでは Python の値との変換と、誤った使い方のエラーだけを扱う。
//! 大きな入力やファイルを読んでいる間は GIL を手放し、ほかのスレッドが動けるようにする。

use std::path::{Path, PathBuf};

use kura_parts::file_hash::{self, Algorithm};
use pyo3::exceptions::{PyBufferError, PyOSError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyMemoryView, PySlice, PyString};

/// これより短い入力は GIL を持ったまま計算する（手放す手間のほうが大きい。hashlib も 2 KiB 未満は手放さない）
const DETACH_THRESHOLD: usize = 2048;
/// 書き換えられるバッファ（bytearray など）は、この大きさずつ bytes に写してから GIL を手放して計算する
const COPY_CHUNK: usize = 1 << 20;

fn algorithm(name: &str) -> PyResult<Algorithm> {
    name.parse()
        .map_err(|e: file_hash::ParseError| PyValueError::new_err(e.to_string()))
}

/// io::Error を、errno とファイル名つきの OSError（FileNotFoundError などの子クラス）にする。
fn os_error(err: std::io::Error, path: &Path) -> PyErr {
    match err.raw_os_error() {
        Some(errno) => {
            // Rust の文言の末尾の " (os error 2)" は、Python では errno として別に出るので外す
            let message = err.to_string();
            let message = message
                .strip_suffix(&format!(" (os error {errno})"))
                .unwrap_or(&message)
                .to_string();
            PyOSError::new_err((errno, message, path.as_os_str().to_os_string()))
        }
        None => PyErr::from(err),
    }
}

/// bytes / bytearray / memoryview（そのほか連続したバッファ）の中身を hasher に入れる。
fn feed(py: Python<'_>, hasher: &mut file_hash::Hasher, data: &Bound<'_, PyAny>) -> PyResult<()> {
    if let Ok(bytes) = data.cast::<PyBytes>() {
        // bytes は書き換えられないので、写さずにそのまま読める
        let slice = bytes.as_bytes();
        if slice.len() < DETACH_THRESHOLD {
            hasher.update(slice);
        } else {
            py.detach(|| {
                hasher.update(slice);
            });
        }
        return Ok(());
    }
    if data.is_instance_of::<PyString>() {
        return Err(PyTypeError::new_err(
            "Strings must be encoded before hashing (e.g. text.encode())",
        ));
    }
    // bytearray・memoryview・array.array などバッファを持つもの。バイト単位の平らな並びとして見る（hashlib と同じ）
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
    if view.getattr("nbytes")?.extract::<usize>()? == 0 {
        // 空の多次元配列（形に 0 を含むもの）は cast できないが、入れるものもない
        return Ok(());
    }
    let view = if view.getattr("format")?.extract::<String>()? == "B"
        && view.getattr("ndim")?.extract::<usize>()? == 1
    {
        view.into_any()
    } else {
        view.call_method1("cast", ("B",))?
    };
    let len: usize = view.len()?;
    // 書き換えられうるバッファは、GIL を持っている間に少しずつ bytes に写し、写したものを GIL なしで計算する
    let mut start = 0;
    while start < len {
        let end = (start + COPY_CHUNK).min(len);
        let piece = view
            .get_item(PySlice::new(py, start as isize, end as isize, 1))?
            .call_method0("tobytes")?;
        let piece = piece.cast::<PyBytes>()?.as_bytes();
        if piece.len() < DETACH_THRESHOLD {
            hasher.update(piece);
        } else {
            py.detach(|| {
                hasher.update(piece);
            });
        }
        start = end;
    }
    Ok(())
}

/// A 32-byte digest, tagged with the algorithm that produced it.
///
/// `str(digest)` is the lowercase hex, same as hashlib's `hexdigest()`.
/// Comparing two digests takes the same time wherever they differ.
#[pyclass(
    module = "kura_rs.file_hash",
    name = "Digest",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Digest {
    inner: file_hash::Digest,
}

#[pymethods]
impl Digest {
    /// Parses 64 hex digits (e.g. a published checksum).
    #[staticmethod]
    fn from_hex(algorithm: &str, hex: &str) -> PyResult<Self> {
        let inner = file_hash::Digest::from_hex(self::algorithm(algorithm)?, hex)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// `"sha256"` or `"blake3"`.
    #[getter]
    fn algorithm(&self) -> &'static str {
        self.inner.algorithm().name()
    }

    /// Always 32.
    #[getter]
    fn digest_size(&self) -> usize {
        file_hash::DIGEST_LEN
    }

    /// The raw 32 bytes, same as hashlib's `digest()`.
    fn digest<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.as_bytes())
    }

    /// Lowercase hex (64 characters), same as hashlib's `hexdigest()`.
    fn hexdigest(&self) -> String {
        self.inner.to_hex()
    }

    /// Alias of `hexdigest()`.
    fn hex(&self) -> String {
        self.inner.to_hex()
    }

    fn __bytes__<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        self.digest(py)
    }

    fn __str__(&self) -> String {
        self.inner.to_hex()
    }

    fn __repr__(&self) -> String {
        format!(
            "Digest('{}', '{}')",
            self.inner.algorithm(),
            self.inner.to_hex()
        )
    }
}

/// An incremental hasher, like `hashlib.sha256()`: feed pieces with `update`,
/// read the result with `finalize` / `digest` / `hexdigest` (any number of times).
#[pyclass(module = "kura_rs.file_hash", name = "Hasher")]
pub struct Hasher {
    inner: file_hash::Hasher,
}

#[pymethods]
impl Hasher {
    #[new]
    #[pyo3(signature = (algorithm = "sha256", data = None))]
    fn new(py: Python<'_>, algorithm: &str, data: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut inner = file_hash::Hasher::new(self::algorithm(algorithm)?);
        if let Some(data) = data {
            feed(py, &mut inner, data)?;
        }
        Ok(Self { inner })
    }

    /// Appends bytes, bytearray or memoryview.
    fn update(&mut self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        feed(py, &mut self.inner, data)
    }

    /// Reads a file to the end and appends its contents. Returns the number of bytes read.
    fn update_file(&mut self, py: Python<'_>, path: PathBuf) -> PyResult<u64> {
        let inner = &mut self.inner;
        py.detach(|| std::fs::File::open(&path).and_then(|f| inner.update_reader(f)))
            .map_err(|e| os_error(e, &path))
    }

    /// The digest of everything fed so far. More input can still be added afterwards.
    fn finalize(&self) -> Digest {
        Digest {
            inner: self.inner.finalize(),
        }
    }

    fn digest<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.finalize().as_bytes())
    }

    fn hexdigest(&self) -> String {
        self.inner.finalize().to_hex()
    }

    /// An independent copy of the current state.
    fn copy(&self) -> Hasher {
        Hasher {
            inner: self.inner.clone(),
        }
    }

    /// Forgets all input.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// `"sha256"` or `"blake3"`, like hashlib's `name`.
    #[getter]
    fn name(&self) -> &'static str {
        self.inner.algorithm().name()
    }

    #[getter]
    fn digest_size(&self) -> usize {
        file_hash::DIGEST_LEN
    }

    fn __repr__(&self) -> String {
        format!("Hasher('{}')", self.inner.algorithm())
    }
}

/// The digest of bytes, bytearray or memoryview.
#[pyfunction]
fn hash_bytes(py: Python<'_>, algorithm: &str, data: &Bound<'_, PyAny>) -> PyResult<Digest> {
    let mut hasher = file_hash::Hasher::new(self::algorithm(algorithm)?);
    feed(py, &mut hasher, data)?;
    Ok(Digest {
        inner: hasher.finalize(),
    })
}

/// The digest of a file's contents (str or os.PathLike), streamed with constant memory.
#[pyfunction]
fn hash_file(py: Python<'_>, algorithm: &str, path: PathBuf) -> PyResult<Digest> {
    let algorithm = self::algorithm(algorithm)?;
    let inner = py
        .detach(|| file_hash::hash_file(algorithm, &path))
        .map_err(|e| os_error(e, &path))?;
    Ok(Digest { inner })
}

/// Digests of many files, in order. Stops at the first file that cannot be read.
#[pyfunction]
fn hash_files(py: Python<'_>, algorithm: &str, paths: Vec<PathBuf>) -> PyResult<Vec<Digest>> {
    let algorithm = self::algorithm(algorithm)?;
    py.detach(|| {
        paths
            .iter()
            .map(|path| file_hash::hash_file(algorithm, path).map_err(|e| (e, path.clone())))
            .collect::<Result<Vec<_>, _>>()
    })
    .map(|digests| digests.into_iter().map(|inner| Digest { inner }).collect())
    .map_err(|(e, path)| os_error(e, &path))
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "file_hash")?;
    m.add_class::<Digest>()?;
    m.add_class::<Hasher>()?;
    m.add_function(wrap_pyfunction!(hash_bytes, &m)?)?;
    m.add_function(wrap_pyfunction!(hash_file, &m)?)?;
    m.add_function(wrap_pyfunction!(hash_files, &m)?)?;
    m.add(
        "ALGORITHMS",
        (Algorithm::Sha256.name(), Algorithm::Blake3.name()),
    )?;
    m.add("DIGEST_SIZE", file_hash::DIGEST_LEN)?;
    parent.add_submodule(&m)
}
