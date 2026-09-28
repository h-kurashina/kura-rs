//! `kura_rs.multi_pattern_match`：parts/multi_pattern_match を Python から使うための薄い包み。
//! 計算は Rust 側に任せ、ここでは Python の値との変換と、誤った使い方のエラーだけを扱う。
//!
//! パターンがすべて str なら「文字列モード」：入力も str で、位置は文字（コードポイント）単位
//! （pyahocorasick の iter() と同じ）。中では UTF-8 のバイト列として探し、位置を文字単位に直す。
//! パターンがすべて bytes 系なら「バイト列モード」：入力も bytes 系で、位置はバイト単位。
//! 探している間は GIL を手放し、ほかのスレッドが動けるようにする。

use kura_parts::multi_pattern_match::{self, Match};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView, PyString, PyTuple};

/// これより短い入力は GIL を持ったまま探す（手放す手間のほうが大きい）
const DETACH_THRESHOLD: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// パターンが 1 つもない（どちらの入力も受け付け、何も見つからない）
    Empty,
    Text,
    Bytes,
}

/// 入力の中身。bytes はそのまま借り、それ以外は写す
enum Haystack<'py> {
    Borrowed(Bound<'py, PyBytes>),
    Owned(Vec<u8>),
}

impl Haystack<'_> {
    fn as_slice(&self) -> &[u8] {
        match self {
            Haystack::Borrowed(b) => b.as_bytes(),
            Haystack::Owned(v) => v,
        }
    }
}

fn type_name(obj: &Bound<'_, PyAny>) -> String {
    obj.get_type()
        .name()
        .map(|n| n.to_string())
        .unwrap_or_default()
}

/// bytes / bytearray / memoryview などバッファを持つものを、バイト列として読む
fn buffer_bytes<'py>(obj: &Bound<'py, PyAny>, what: &str) -> PyResult<Haystack<'py>> {
    if let Ok(bytes) = obj.cast::<PyBytes>() {
        return Ok(Haystack::Borrowed(bytes.clone()));
    }
    let view = PyMemoryView::from(obj).map_err(|_| {
        PyTypeError::new_err(format!(
            "{what} must be str or a bytes-like object, not '{}'",
            type_name(obj)
        ))
    })?;
    let bytes = view.call_method0("tobytes")?;
    Ok(Haystack::Owned(
        bytes.cast::<PyBytes>()?.as_bytes().to_vec(),
    ))
}

/// Many patterns compiled into one Aho–Corasick automaton. Build once, search many haystacks.
///
/// Patterns must be all `str` (search `str`, offsets in characters, like pyahocorasick)
/// or all bytes-like (search bytes-like, offsets in bytes).
#[pyclass(module = "kura_rs.multi_pattern_match", name = "Matcher", frozen)]
pub struct Matcher {
    inner: multi_pattern_match::Matcher,
    mode: Mode,
    /// 渡されたパターン（str か bytes のタプル）
    patterns: Py<PyTuple>,
    /// 文字列モードで、パターンごとの文字数（開始位置を文字単位に直すのに使う）
    char_lens: Vec<usize>,
}

impl Matcher {
    /// 入力を、このマッチャーのモードに合ったバイト列にする
    fn haystack<'py>(&self, haystack: &Bound<'py, PyAny>) -> PyResult<(Haystack<'py>, bool)> {
        if let Ok(s) = haystack.cast::<PyString>() {
            if self.mode == Mode::Bytes {
                return Err(PyTypeError::new_err(
                    "this matcher was built from bytes patterns, so the haystack must be bytes-like (encode the str first)",
                ));
            }
            let text = s.to_cow()?;
            // ASCII だけなら、バイト位置と文字位置が同じなので直さなくてよい
            let ascii = text.is_ascii();
            return Ok((Haystack::Owned(text.into_owned().into_bytes()), !ascii));
        }
        if self.mode == Mode::Text {
            return Err(PyTypeError::new_err(format!(
                "this matcher was built from str patterns, so the haystack must be str, not '{}'",
                type_name(haystack)
            )));
        }
        Ok((buffer_bytes(haystack, "haystack")?, false))
    }

    /// 探して、(pattern, start, end) の列にする。needs_chars なら位置を文字単位に直す
    fn search(
        &self,
        py: Python<'_>,
        haystack: &Bound<'_, PyAny>,
        find: impl Fn(&[u8]) -> Vec<Match> + Send + Sync,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        let (data, needs_chars) = self.haystack(haystack)?;
        let bytes = data.as_slice();
        let run = || {
            let matches = find(bytes);
            if needs_chars {
                to_char_offsets(bytes, &matches, &self.char_lens)
            } else {
                matches
                    .iter()
                    .map(|m| (m.pattern, m.start, m.end))
                    .collect()
            }
        };
        Ok(if bytes.len() < DETACH_THRESHOLD {
            run()
        } else {
            py.detach(run)
        })
    }
}

/// バイト位置を文字位置に直す。一致は end の順に並んでいるので、先頭から一度なめるだけでよい。
/// 開始位置は「終わりの文字位置 − パターンの文字数」（UTF-8 の一致は必ず文字の境目にある）。
fn to_char_offsets(
    haystack: &[u8],
    matches: &[Match],
    char_lens: &[usize],
) -> Vec<(usize, usize, usize)> {
    let mut byte_pos = 0;
    let mut char_pos = 0;
    matches
        .iter()
        .map(|m| {
            debug_assert!(m.end >= byte_pos);
            // 続きのバイト（0b10xxxxxx）以外を数えると文字数になる
            char_pos += haystack[byte_pos..m.end]
                .iter()
                .filter(|&&b| (b as i8) >= -0x40)
                .count();
            byte_pos = m.end;
            (m.pattern, char_pos - char_lens[m.pattern], char_pos)
        })
        .collect()
}

fn to_list<'py>(
    py: Python<'py>,
    matches: Vec<(usize, usize, usize)>,
) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, matches)
}

#[pymethods]
impl Matcher {
    #[new]
    fn new(py: Python<'_>, patterns: &Bound<'_, PyAny>) -> PyResult<Self> {
        if patterns.is_instance_of::<PyString>() || patterns.is_instance_of::<PyBytes>() {
            return Err(PyTypeError::new_err(
                "pass an iterable of patterns (e.g. a list), not a single str/bytes",
            ));
        }
        let mut mode = Mode::Empty;
        let mut kept: Vec<Bound<'_, PyAny>> = Vec::new();
        let mut raw: Vec<Vec<u8>> = Vec::new();
        let mut char_lens = Vec::new();
        for (i, item) in patterns.try_iter()?.enumerate() {
            let item = item?;
            let this = if item.is_instance_of::<PyString>() {
                Mode::Text
            } else {
                Mode::Bytes
            };
            if mode != Mode::Empty && mode != this {
                return Err(PyTypeError::new_err(format!(
                    "patterns must be all str or all bytes-like; pattern {i} is '{}'",
                    type_name(&item)
                )));
            }
            mode = this;
            if let Ok(s) = item.cast::<PyString>() {
                let text = s.to_cow()?;
                char_lens.push(text.chars().count());
                raw.push(text.into_owned().into_bytes());
                kept.push(item);
            } else {
                let bytes = buffer_bytes(&item, &format!("pattern {i}"))?;
                let bytes = bytes.as_slice().to_vec();
                kept.push(PyBytes::new(py, &bytes).into_any());
                raw.push(bytes);
            }
        }
        let inner = py
            .detach(|| multi_pattern_match::Matcher::new(&raw))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner,
            mode,
            patterns: PyTuple::new(py, kept)?.unbind(),
            char_lens,
        })
    }

    /// Every occurrence of every pattern, overlapping ones included, as
    /// `(pattern_id, start, end)` tuples ordered by `end`, then longest first
    /// (the order of pyahocorasick's `Automaton.iter()`). `end` is exclusive.
    /// With `limit`, stops after that many matches.
    #[pyo3(signature = (haystack, *, limit = None))]
    fn find_all<'py>(
        &self,
        py: Python<'py>,
        haystack: &Bound<'py, PyAny>,
        limit: Option<usize>,
    ) -> PyResult<Bound<'py, PyList>> {
        let limit = limit.unwrap_or(usize::MAX);
        let matches = self.search(py, haystack, |h| {
            self.inner.find_overlapping_iter(h).take(limit).collect()
        })?;
        to_list(py, matches)
    }

    /// Non-overlapping matches, leftmost first and, at the same start, the longest.
    fn find_longest<'py>(
        &self,
        py: Python<'py>,
        haystack: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyList>> {
        let matches = self.search(py, haystack, |h| self.inner.find_longest(h))?;
        to_list(py, matches)
    }

    /// Number of overlapping matches (same as `len(find_all(haystack))`, without building the list).
    fn count(&self, py: Python<'_>, haystack: &Bound<'_, PyAny>) -> PyResult<usize> {
        let (data, _) = self.haystack(haystack)?;
        let bytes = data.as_slice();
        Ok(if bytes.len() < DETACH_THRESHOLD {
            self.inner.count_overlapping(bytes)
        } else {
            py.detach(|| self.inner.count_overlapping(bytes))
        })
    }

    /// Whether any pattern occurs in the haystack. Stops at the first match.
    fn is_match(&self, py: Python<'_>, haystack: &Bound<'_, PyAny>) -> PyResult<bool> {
        let (data, _) = self.haystack(haystack)?;
        let bytes = data.as_slice();
        Ok(if bytes.len() < DETACH_THRESHOLD {
            self.inner.is_match(bytes)
        } else {
            py.detach(|| self.inner.is_match(bytes))
        })
    }

    /// The patterns as given (`str`, or `bytes` for bytes-like ones), indexed by pattern id.
    #[getter]
    fn patterns<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        self.patterns.bind(py).clone()
    }

    /// `"str"`, `"bytes"`, or `"empty"` when there are no patterns (then any haystack is accepted).
    #[getter]
    fn kind(&self) -> &'static str {
        match self.mode {
            Mode::Empty => "empty",
            Mode::Text => "str",
            Mode::Bytes => "bytes",
        }
    }

    /// Heap memory used by the automaton, in bytes.
    #[getter]
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    fn __len__(&self) -> usize {
        self.inner.pattern_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "Matcher({} {} patterns)",
            self.inner.pattern_count(),
            self.kind()
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "multi_pattern_match")?;
    m.add_class::<Matcher>()?;
    parent.add_submodule(&m)
}
