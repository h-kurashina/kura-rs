//! `kura_rs.unicode_normalize`：parts/unicode_normalize を Python から使うための薄い包み。
//! 計算は Rust 側に任せ、ここでは Python の値との変換と、誤った使い方のエラーだけを扱う。
//! 長い文字列を処理している間は GIL を手放し、ほかのスレッドが動けるようにする。
//! 何も変わらなかったときは、受け取った str をそのまま返す（unicodedata.normalize と同じ）。

use std::borrow::Cow;

use kura_parts::unicode_normalize::{self as un, Form, Options};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyTuple};

/// これより短い文字列（UTF-8 のバイト数）は GIL を持ったまま処理する（手放す手間のほうが大きい）
const DETACH_THRESHOLD: usize = 4096;

fn options(form: Option<&str>, fold_whitespace: bool, strip_invisible: bool) -> PyResult<Options> {
    let form = form
        .map(|name| {
            name.parse::<Form>()
                .map_err(|e| PyValueError::new_err(e.to_string()))
        })
        .transpose()?;
    Ok(Options {
        form,
        fold_whitespace,
        strip_invisible,
    })
}

/// 変わらなければ None、変わったら新しい文字列。
fn apply(text: &str, options: Options) -> Option<String> {
    match un::normalize(text, options) {
        Cow::Borrowed(_) => None,
        Cow::Owned(s) => Some(s),
    }
}

/// 結果を Python の str にする。変わらなければ受け取った str（str の子クラスなら str に直したもの）を返す。
fn to_python<'py>(
    text: &Bound<'py, PyString>,
    utf8: &str,
    out: Option<String>,
) -> Bound<'py, PyString> {
    match out {
        Some(s) => PyString::new(text.py(), &s),
        None if text.is_exact_instance_of::<PyString>() => text.clone(),
        None => PyString::new(text.py(), utf8),
    }
}

fn run<'py>(text: &Bound<'py, PyString>, options: Options) -> PyResult<Bound<'py, PyString>> {
    // サロゲートを含む str は UTF-8 にできないので UnicodeEncodeError になる
    let utf8 = text.to_cow()?;
    let out = if utf8.len() < DETACH_THRESHOLD {
        apply(&utf8, options)
    } else {
        text.py().detach(|| apply(&utf8, options))
    };
    Ok(to_python(text, &utf8, out))
}

/// Cleans up `text`: removes invisible characters (if `strip_invisible`),
/// normalizes to `form` ("NFC", "NFD", "NFKC", "NFKD" or None), then folds
/// whitespace (if `fold_whitespace`). With only `form`, the result is the same
/// as `unicodedata.normalize(form, text)`.
#[pyfunction]
#[pyo3(signature = (text, form = Some("NFC"), *, fold_whitespace = false, strip_invisible = false))]
fn normalize<'py>(
    text: &Bound<'py, PyString>,
    form: Option<&str>,
    fold_whitespace: bool,
    strip_invisible: bool,
) -> PyResult<Bound<'py, PyString>> {
    run(text, options(form, fold_whitespace, strip_invisible)?)
}

/// `normalize` for many strings at once (the GIL is released for the whole batch). Returns a list.
#[pyfunction]
#[pyo3(signature = (texts, form = Some("NFC"), *, fold_whitespace = false, strip_invisible = false))]
fn normalize_many<'py>(
    py: Python<'py>,
    texts: &Bound<'py, PyAny>,
    form: Option<&str>,
    fold_whitespace: bool,
    strip_invisible: bool,
) -> PyResult<Bound<'py, PyList>> {
    let options = options(form, fold_whitespace, strip_invisible)?;
    if texts.is_instance_of::<PyString>() {
        return Err(PyTypeError::new_err(
            "pass an iterable of str (e.g. a list), not a single str; use normalize() for one string",
        ));
    }
    let mut objects = Vec::new();
    let mut utf8s = Vec::new();
    for item in texts.try_iter()? {
        let item = item?;
        let text = item.cast_into::<PyString>().map_err(|e| {
            PyTypeError::new_err(format!(
                "items must be str, not {}",
                e.into_inner()
                    .get_type()
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ))
        })?;
        utf8s.push(text.to_cow()?.into_owned());
        objects.push(text);
    }
    let outs: Vec<Option<String>> =
        py.detach(|| utf8s.iter().map(|utf8| apply(utf8, options)).collect());
    let results = objects
        .iter()
        .zip(&utf8s)
        .zip(outs)
        .map(|((text, utf8), out)| to_python(text, utf8, out));
    PyList::new(py, results)
}

/// True if `text` is already in normalization form `form`, the same as
/// `unicodedata.is_normalized(form, text)` (note the argument order).
#[pyfunction]
#[pyo3(signature = (text, form = "NFC"))]
fn is_normalized(py: Python<'_>, text: &Bound<'_, PyString>, form: &str) -> PyResult<bool> {
    let form = form
        .parse::<Form>()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let utf8 = text.to_cow()?;
    Ok(if utf8.len() < DETACH_THRESHOLD {
        un::is_normalized(&utf8, form)
    } else {
        py.detach(|| un::is_normalized(&utf8, form))
    })
}

/// Replaces every run of whitespace (`str.isspace()`) with one space and trims
/// both ends, the same as `" ".join(text.split())`.
#[pyfunction]
fn fold_whitespace<'py>(text: &Bound<'py, PyString>) -> PyResult<Bound<'py, PyString>> {
    run(text, Options::new().fold_whitespace())
}

/// Removes the invisible characters listed in `INVISIBLE` (format characters
/// that are default-ignorable: ZWSP, ZWNJ, ZWJ, BOM, soft hyphen, bidi controls, tags, ...).
#[pyfunction]
fn strip_invisible<'py>(text: &Bound<'py, PyString>) -> PyResult<Bound<'py, PyString>> {
    run(text, Options::new().strip_invisible())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "unicode_normalize")?;
    m.add_function(wrap_pyfunction!(normalize, &m)?)?;
    m.add_function(wrap_pyfunction!(normalize_many, &m)?)?;
    m.add_function(wrap_pyfunction!(is_normalized, &m)?)?;
    m.add_function(wrap_pyfunction!(fold_whitespace, &m)?)?;
    m.add_function(wrap_pyfunction!(strip_invisible, &m)?)?;
    let [nfc, nfd, nfkc, nfkd] = Form::ALL.map(Form::name);
    m.add("FORMS", (nfc, nfd, nfkc, nfkd))?;
    let (major, minor, update) = un::UNICODE_VERSION;
    m.add("UNICODE_VERSION", format!("{major}.{minor}.{update}"))?;
    m.add("WHITESPACE", un::WHITESPACE.iter().collect::<String>())?;
    let invisible = un::INVISIBLE.map(|(lo, hi)| (lo as u32, hi as u32));
    m.add("INVISIBLE", PyTuple::new(parent.py(), invisible)?)?;
    parent.add_submodule(&m)
}
