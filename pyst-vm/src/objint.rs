use super::pyobject::{PyFuncArgs, PyObject, PyObjectKind, PyObjectRef};
use super::vm::VirtualMachine;
use std::collections::HashMap;

fn str(
  vm: &mut VirtualMachine,
  _args: PyFuncArgs,
) -> Result<PyObjectRef, PyObjectRef> {
  Ok(vm.new_str("todo".to_string()))
}

pub fn create_type(type_type: PyObjectRef) -> PyObjectRef {
  let mut dict = HashMap::new();
  dict.insert(
    "__str__".to_string(),
    PyObject::new(
      PyObjectKind::RustFunction { function: str },
      type_type.clone(),
    ),
  );
  let typ = PyObject::new(
    PyObjectKind::Class {
      name: "int".to_string(),
      dict: PyObject::new(
        PyObjectKind::Dict { elements: dict },
        type_type.clone(),
      ),
      mro: vec![],
    },
    type_type.clone(),
  );
  typ
}
