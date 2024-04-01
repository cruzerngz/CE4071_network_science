//! dblp is a library for parsing and querying the DBLP XML file.

mod dataset;

use pyo3::{exceptions::PyTypeError, prelude::*};

/// Initialize the DBLP database from a local file.
/// This function must be called before performing other actions!
#[pyfunction]
pub fn init(path: String) -> PyResult<()> {
    if dataset::DBLP_DATABASE.get().is_some() {
        return Ok(());
    }

    match dataset::Dblp::new(path) {
        Ok(dblp) => dataset::DBLP_DATABASE.get_or_init(|| dblp),
        Err(e) => return Err(PyTypeError::new_err(e)),
    };

    Ok(())
}

/// Query the DBLP database
#[pyfunction]
pub fn query() -> PyResult<()> {
    println!("Hwllo world!");

    Ok(())
}

/// dblp is a library for parsing and querying the DBLP XML file.
#[pymodule]
fn dblp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(query, m)?)?;
    Ok(())
}
