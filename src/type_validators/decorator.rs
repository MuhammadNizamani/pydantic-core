use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyAny, PyDict};

use super::{TypeValidator, ValError, ValidationError};
use crate::errors::ValResult;
use crate::type_validators::build_type_validator;
use crate::utils::{dict_get_required, py_error};

#[derive(Debug, Clone)]
pub struct PreDecoratorValidator {
    validator: Box<dyn TypeValidator>,
    func: PyObject,
}

impl TypeValidator for PreDecoratorValidator {
    fn is_match(type_: &str, dict: &PyDict) -> bool {
        type_ == "decorator" && dict.get_item("pre_decorator").is_some()
    }

    fn build(dict: &PyDict) -> PyResult<Self> {
        Ok(Self {
            validator: build_type_validator(dict_get_required!(dict, "field", &PyDict)?)?,
            func: get_function(dict, "pre_decorator")?,
        })
    }

    fn validate(&self, py: Python, obj: &PyAny) -> ValResult<PyObject> {
        let value = match self.func.call(py, (obj,), None) {
            Ok(output) => Ok(output),
            // TODO this is wrong, we should check for errors which could as validation errors
            Err(err) => Err(ValError::InternalErr(err)),
        }?;
        let v: &PyAny = value.as_ref(py);
        self.validator.validate(py, v)
    }

    fn clone_dyn(&self) -> Box<dyn TypeValidator> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PostDecoratorValidator {
    validator: Box<dyn TypeValidator>,
    func: PyObject,
}

impl TypeValidator for PostDecoratorValidator {
    fn is_match(type_: &str, dict: &PyDict) -> bool {
        type_ == "decorator" && dict.get_item("post_decorator").is_some()
    }

    fn build(dict: &PyDict) -> PyResult<Self> {
        Ok(Self {
            validator: build_type_validator(dict_get_required!(dict, "field", &PyDict)?)?,
            func: get_function(dict, "post_decorator")?,
        })
    }

    fn validate(&self, py: Python, obj: &PyAny) -> ValResult<PyObject> {
        let v = self.validator.validate(py, obj)?;
        match self.func.call(py, (v,), None) {
            Ok(output) => Ok(output),
            // TODO this is wrong, we should check for errors which could as validation errors
            Err(err) => Err(ValError::InternalErr(err)),
        }
    }

    fn clone_dyn(&self) -> Box<dyn TypeValidator> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct WrapDecoratorValidator {
    validator: Box<dyn TypeValidator>,
    func: PyObject,
}

impl TypeValidator for WrapDecoratorValidator {
    fn is_match(type_: &str, dict: &PyDict) -> bool {
        type_ == "decorator" && dict.get_item("wrap_decorator").is_some()
    }

    fn build(dict: &PyDict) -> PyResult<Self> {
        Ok(Self {
            validator: build_type_validator(dict_get_required!(dict, "field", &PyDict)?)?,
            func: get_function(dict, "wrap_decorator")?,
        })
    }

    fn validate(&self, py: Python, obj: &PyAny) -> ValResult<PyObject> {
        let validator_kwarg = ValidatorCallable {
            type_validator: self.validator.clone(),
        };
        let kwargs = [("validator", validator_kwarg.into_py(py))];
        match self.func.call(py, (obj,), Some(kwargs.into_py_dict(py))) {
            Ok(output) => Ok(output),
            // TODO this is wrong, we should check for errors which could as validation errors
            Err(err) => Err(ValError::InternalErr(err)),
        }
    }

    fn clone_dyn(&self) -> Box<dyn TypeValidator> {
        Box::new(self.clone())
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ValidatorCallable {
    type_validator: Box<dyn TypeValidator>,
}

#[pymethods]
impl ValidatorCallable {
    fn __call__(&self, py: Python, arg: &PyAny) -> PyResult<PyObject> {
        match self.type_validator.validate(py, arg) {
            Ok(obj) => Ok(obj),
            Err(ValError::LineErrors(line_errors)) => Err(ValidationError::new_err((line_errors, "Model".to_string()))),
            Err(ValError::InternalErr(err)) => Err(err),
        }
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("ValidatorCallable({:?})", self.type_validator))
    }
}

fn get_function(dict: &PyDict, key: &str) -> PyResult<PyObject> {
    match dict.get_item(key) {
        Some(obj) => {
            if !obj.is_callable() {
                return py_error!(r#""{}" must be callable"#, key);
            }
            Ok(obj.to_object(obj.py()))
        }
        None => py_error!(r#""{}" is required"#, key),
    }
}
