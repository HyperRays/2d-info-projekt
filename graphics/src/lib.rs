use pyo3::prelude::*;
use game_eng::run;

/// runs the wgpu bindings
#[pyfunction]
fn run_bind(){
    run();
}

/// A Python module implemented in Rust.
#[pymodule]
fn graphics(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_bind, m)?)?;
    Ok(())
}