//! `kura_rs.minhash`：parts/minhash を Python から使うための薄い包み。
//! 計算は Rust 側に任せ、ここでは Python の値との変換と、誤った使い方のエラーだけを扱う。

use kura_parts::minhash;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyString};

/// Python の bytes / bytearray / str を、ハッシュにかけるバイト列にする（str は UTF-8）。
fn to_bytes(item: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(b) = item.cast::<PyBytes>() {
        Ok(b.as_bytes().to_vec())
    } else if let Ok(b) = item.cast::<PyByteArray>() {
        Ok(b.to_vec())
    } else if let Ok(s) = item.cast::<PyString>() {
        Ok(s.to_cow()?.as_bytes().to_vec())
    } else {
        Err(PyTypeError::new_err(format!(
            "items must be bytes, bytearray or str, not {}",
            item.get_type().name()?
        )))
    }
}

fn collect(items: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<u8>>> {
    if items.is_instance_of::<PyBytes>() || items.is_instance_of::<PyString>() {
        return Err(PyTypeError::new_err(
            "pass an iterable of items (e.g. a list of tokens), not a single bytes/str",
        ));
    }
    items.try_iter()?.map(|item| to_bytes(&item?)).collect()
}

/// The permutation functions. Build once and reuse for every document.
///
/// Same `num_perm` and `seed` as `datasketch.MinHash(num_perm, seed)` give identical signatures.
#[pyclass(module = "kura_rs.minhash", frozen)]
pub struct MinHasher {
    inner: minhash::MinHasher,
}

#[pymethods]
impl MinHasher {
    #[new]
    #[pyo3(signature = (num_perm = 128, seed = 1))]
    fn new(num_perm: i64, seed: i64) -> PyResult<Self> {
        if num_perm < 1 {
            return Err(PyValueError::new_err("num_perm must be positive"));
        }
        let num_perm = usize::try_from(num_perm)
            .map_err(|_| PyValueError::new_err("num_perm is too large"))?;
        let seed = u32::try_from(seed)
            .map_err(|_| PyValueError::new_err("seed must be in 0..=2**32-1"))?;
        Ok(Self {
            inner: minhash::MinHasher::new(num_perm, seed),
        })
    }

    #[getter]
    fn num_perm(&self) -> usize {
        self.inner.num_perm()
    }

    /// `(a, b)`: the permutation parameters, same as `datasketch.MinHash(...).permutations`.
    #[getter]
    fn permutations(&self) -> (Vec<u32>, Vec<u32>) {
        let (a, b) = self.inner.permutations();
        (a.to_vec(), b.to_vec())
    }

    /// An empty signature, to fill with `Signature.update`.
    fn empty(&self) -> Signature {
        Signature {
            inner: self.inner.empty(),
        }
    }

    /// The signature of an iterable of items (bytes, bytearray or str).
    fn signature(&self, py: Python<'_>, items: &Bound<'_, PyAny>) -> PyResult<Signature> {
        let items = collect(items)?;
        let inner = py.detach(|| self.inner.signature(&items));
        Ok(Signature { inner })
    }

    /// Signatures of many documents at once (each an iterable of items).
    fn signatures(&self, py: Python<'_>, documents: &Bound<'_, PyAny>) -> PyResult<Vec<Signature>> {
        let docs: Vec<Vec<Vec<u8>>> = documents
            .try_iter()?
            .map(|d| collect(&d?))
            .collect::<PyResult<_>>()?;
        let sigs = py.detach(|| {
            docs.iter()
                .map(|d| self.inner.signature(d))
                .collect::<Vec<_>>()
        });
        Ok(sigs.into_iter().map(|inner| Signature { inner }).collect())
    }

    fn __repr__(&self) -> String {
        format!("MinHasher(num_perm={})", self.inner.num_perm())
    }
}

/// A MinHash signature.
#[pyclass(module = "kura_rs.minhash", eq)]
#[derive(PartialEq)]
pub struct Signature {
    inner: minhash::Signature,
}

impl Signature {
    fn same_length(&self, n: usize, what: &str) -> PyResult<()> {
        if self.inner.values().len() == n {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "{what} has num_perm={n}, but this signature has num_perm={}",
                self.inner.values().len()
            )))
        }
    }
}

#[pymethods]
impl Signature {
    /// Adds one item. `hasher` must be the MinHasher that created this signature.
    fn update(&mut self, hasher: &MinHasher, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.same_length(hasher.inner.num_perm(), "hasher")?;
        self.inner.update(&hasher.inner, &to_bytes(item)?);
        Ok(())
    }

    /// Adds many items.
    fn update_batch(
        &mut self,
        py: Python<'_>,
        hasher: &MinHasher,
        items: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.same_length(hasher.inner.num_perm(), "hasher")?;
        let items = collect(items)?;
        let sig = &mut self.inner;
        py.detach(|| {
            items
                .iter()
                .for_each(|item| sig.update(&hasher.inner, item))
        });
        Ok(())
    }

    /// Estimated Jaccard similarity with another signature from the same MinHasher.
    fn jaccard(&self, other: &Signature) -> PyResult<f64> {
        self.same_length(other.inner.values().len(), "other")?;
        Ok(self.inner.jaccard(&other.inner))
    }

    /// Merges `other` into this signature (the signature of the union).
    fn merge(&mut self, other: &Signature) -> PyResult<()> {
        self.same_length(other.inner.values().len(), "other")?;
        self.inner.merge(&other.inner);
        Ok(())
    }

    /// The signature values, same as `datasketch.MinHash(...).hashvalues`.
    #[getter]
    fn values(&self) -> Vec<u32> {
        self.inner.values().to_vec()
    }

    /// Alias of `values`, named like datasketch.
    #[getter]
    fn hashvalues(&self) -> Vec<u32> {
        self.values()
    }

    fn copy(&self) -> Signature {
        Signature {
            inner: self.inner.clone(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.values().len()
    }

    fn __repr__(&self) -> String {
        format!("Signature(num_perm={})", self.inner.values().len())
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "minhash")?;
    m.add_class::<MinHasher>()?;
    m.add_class::<Signature>()?;
    parent.add_submodule(&m)
}
