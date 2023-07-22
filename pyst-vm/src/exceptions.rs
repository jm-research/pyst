use super::pyobject::{PyFuncArgs, PyObject, PyObjectKind, PyObjectRef, PyResult};
use super::vm::VirtualMachine;
use std::collections::HashMap;

fn init(vm: &mut VirtualMachine, _args: PyFuncArgs) -> PyResult {
  Ok(vm.new_str("todo".to_string()))
}

pub fn create_base_exception_type(
  type_type: PyObjectRef,
  object_type: PyObjectRef,
) -> PyObjectRef {
  let mut dict = HashMap::new();
  dict.insert(
    "__init__".to_string(),
    PyObject::new(
      PyObjectKind::RustFunction { function: init },
      type_type.clone(),
    ),
  );
  let typ = PyObject::new(
    PyObjectKind::Class {
      name: "BaseException".to_string(),
      dict: PyObject::new(
        PyObjectKind::Dict { elements: dict },
        type_type.clone(),
      ),
      mro: vec![object_type],
    },
    type_type.clone(),
  );
  typ
}
