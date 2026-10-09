use crate::json::value::JsonValue;
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::{
    PyResult, pyfunction,
    types::{PyAny, PyDict, PyList},
};
use std::io::Cursor;

#[pymodule]
mod rjson {
    use super::*;
    use crate::json::json_load;

    #[pyfunction]
    fn rjson_load(py: Python<'_>, data: Vec<u8>) -> PyResult<Bound<'_, PyAny>> {
        let reader = Cursor::new(data);
        let json = json_load(reader).map_err(|err| PyValueError::new_err(err.to_string()))?;
        jsonvalue2py(&json, py)
    }

    #[pyfunction]
    fn rjson_loads(py: Python<'_>, string: String) -> PyResult<Bound<'_, PyAny>> {
        let reader = Cursor::new(string);
        let json = json_load(reader).map_err(|err| PyValueError::new_err(err.to_string()))?;
        jsonvalue2py(&json, py)
    }
}

fn jsonvalue2py<'a>(json: &JsonValue, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
    match json {
        JsonValue::Null => Ok(py.None().into_bound(py)),
        JsonValue::True => true.into_bound_py_any(py),
        JsonValue::False => false.into_bound_py_any(py),
        JsonValue::Number(v) => v.into_bound_py_any(py),
        JsonValue::String(v) => v.into_bound_py_any(py),
        JsonValue::Array(items) => {
            let pylist = PyList::empty(py);
            for item in items {
                if let Ok(r) = jsonvalue2py(item, py) {
                    pylist.append(r)?;
                } else {
                    return Err(PyValueError::new_err("expected array"));
                }
            }
            Ok(pylist.into_any())
        }
        JsonValue::Object(o) => {
            let pydict = PyDict::new(py);
            for (k, v) in o.iter() {
                if let Ok(r) = jsonvalue2py(v, py) {
                    pydict.set_item(k.to_string(), r)?;
                } else {
                    return Err(PyValueError::new_err("expected object"));
                }
            }
            Ok(pydict.into_any())
        }
    }
}
