use super::bytecode;
use super::exceptions;
use super::objdict;
use super::objfunction;
use super::objint;
use super::objlist;
use super::objobject;
use super::objtype;
use super::vm::VirtualMachine;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, Div, Mul, Rem, Sub};
use std::rc::Rc;

pub type PyRef<T> = Rc<RefCell<T>>;
pub type PyObjectRef = PyRef<PyObject>;
pub type PyResult = Result<PyObjectRef, PyObjectRef>;

#[derive(Debug)]
pub struct PyContext {
  pub type_type: PyObjectRef,
  pub none_type: PyObjectRef,
  pub dict_type: PyObjectRef,
  pub int_type: PyObjectRef,
  pub list_type: PyObjectRef,
  pub tuple_type: PyObjectRef,
  pub function_type: PyObjectRef,
  pub bound_method_type: PyObjectRef,
  pub member_descriptor_type: PyObjectRef,
  pub object_type: PyObjectRef,
  pub base_exception_type: PyObjectRef,
}

fn _nothing() -> PyObjectRef {
  PyObject {
    kind: PyObjectKind::PyNone,
    typ: None,
  }
  .into_ref()
}

impl PyContext {
  pub fn new() -> PyContext {
    let type_type = _nothing();
    let object_type = _nothing();
    let dict_type = _nothing();

    objtype::create_type(
      type_type.clone(),
      object_type.clone(),
      dict_type.clone(),
    );
    objobject::create_object(
      type_type.clone(),
      object_type.clone(),
      dict_type.clone(),
    );
    objdict::create_type(
      type_type.clone(),
      object_type.clone(),
      dict_type.clone(),
    );

    let function_type = objfunction::create_type(type_type.clone());
    let bound_method_type =
      objfunction::create_bound_method_type(type_type.clone());
    let member_descriptor_type = objfunction::create_member_descriptor_type(
      type_type.clone(),
      object_type.clone(),
    );

    let context = PyContext {
      int_type: objint::create_type(type_type.clone()),
      list_type: objlist::create_type(type_type.clone(), object_type.clone()),
      tuple_type: type_type.clone(),
      dict_type: dict_type.clone(),
      none_type: PyObject::new(PyObjectKind::PyNone, type_type.clone()),
      object_type: object_type.clone(),
      function_type: function_type,
      bound_method_type: bound_method_type,
      member_descriptor_type: member_descriptor_type,
      type_type: type_type.clone(),
      base_exception_type: exceptions::create_base_exception_type(
        type_type.clone(),
        object_type.clone(),
      ),
    };
    objtype::init(&context);
    objlist::init(&context);
    objobject::init(&context);
    objdict::init(&context);
    // TODO: create exception hierarchy here?
    // exceptions::create_zoo(&context);
    context
  }

  pub fn new_int(&self, i: i32) -> PyObjectRef {
    PyObject::new(PyObjectKind::Integer { value: i }, self.int_type.clone())
  }

  pub fn new_float(&self, i: f64) -> PyObjectRef {
    PyObject::new(PyObjectKind::Float { value: i }, self.type_type.clone())
  }

  pub fn new_str(&self, s: String) -> PyObjectRef {
    PyObject::new(PyObjectKind::String { value: s }, self.type_type.clone())
  }

  pub fn new_bool(&self, b: bool) -> PyObjectRef {
    PyObject::new(PyObjectKind::Boolean { value: b }, self.type_type.clone())
  }

  pub fn new_tuple(&self, elements: Vec<PyObjectRef>) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::Tuple { elements: elements },
      self.tuple_type.clone(),
    )
  }

  pub fn new_list(&self, elements: Vec<PyObjectRef>) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::List { elements: elements },
      self.list_type.clone(),
    )
  }

  pub fn new_dict(&self) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::Dict {
        elements: HashMap::new(),
      },
      self.dict_type.clone(),
    )
  }

  pub fn new_scope(&self, parent: Option<PyObjectRef>) -> PyObjectRef {
    let locals = self.new_dict();
    let scope = Scope {
      locals: locals,
      parent: parent,
    };
    PyObject {
      kind: PyObjectKind::Scope { scope: scope },
      typ: None,
    }
    .into_ref()
  }

  pub fn new_module(&self, name: &String, scope: PyObjectRef) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::Module {
        name: name.clone(),
        dict: scope.clone(),
      },
      self.type_type.clone(),
    )
  }

  pub fn new_rustfunc(&self, function: RustPyFunc) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::RustFunction { function: function },
      self.function_type.clone(),
    )
  }

  pub fn new_function(
    &self,
    code_obj: PyObjectRef,
    scope: PyObjectRef,
  ) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::Function {
        code: code_obj,
        scope: scope,
      },
      self.function_type.clone(),
    )
  }

  pub fn new_bound_method(
    &self,
    function: PyObjectRef,
    object: PyObjectRef,
  ) -> PyObjectRef {
    PyObject::new(
      PyObjectKind::BoundMethod {
        function: function,
        object: object,
      },
      self.bound_method_type.clone(),
    )
  }

  pub fn new_member_descriptor(&self, function: RustPyFunc) -> PyObjectRef {
    let dict = self.new_dict();
    dict.set_item(&String::from("function"), self.new_rustfunc(function));
    self.new_instance(dict, self.member_descriptor_type.clone())
  }

  pub fn new_instance(
    &self,
    dict: PyObjectRef,
    class: PyObjectRef,
  ) -> PyObjectRef {
    PyObject::new(PyObjectKind::Instance { dict: dict }, class)
  }
}

#[derive(Debug)]
pub struct Scope {
  pub locals: PyObjectRef,         // Variables
  pub parent: Option<PyObjectRef>, // Parent scope
}

pub struct PyObject {
  pub kind: PyObjectKind,
  pub typ: Option<PyObjectRef>,
}

impl PyObject {
  pub fn new(kind: PyObjectKind, typ: PyObjectRef) -> PyObjectRef {
    PyObject {
      kind: kind,
      typ: Some(typ),
    }
    .into_ref()
  }

  pub fn str(&self) -> String {
    match self.kind {
      PyObjectKind::String { ref value } => value.clone(),
      PyObjectKind::Integer { ref value } => format!("{:?}", value),
      PyObjectKind::Float { ref value } => format!("{:?}", value),
      PyObjectKind::Boolean { ref value } => format!("{:?}", value),
      PyObjectKind::List { ref elements } => format!(
        "[{}]",
        elements
          .iter()
          .map(|elem| elem.borrow().str())
          .collect::<Vec<_>>()
          .join(", ")
      ),
      PyObjectKind::Tuple { ref elements } => {
        if elements.len() == 1 {
          format!("({},)", elements[0].borrow().str())
        } else {
          format!(
            "({})",
            elements
              .iter()
              .map(|elem| elem.borrow().str())
              .collect::<Vec<_>>()
              .join(", ")
          )
        }
      }
      PyObjectKind::Dict { ref elements } => format!(
        "{{ {} }}",
        elements
          .iter()
          .map(|elem| format!("{}: {}", elem.0, elem.1.borrow().str()))
          .collect::<Vec<_>>()
          .join(", ")
      ),
      PyObjectKind::PyNone => String::from("None"),
      PyObjectKind::Class {
        ref name,
        dict: ref _dict,
        mro: _,
      } => format!("<class '{}'>", name),
      PyObjectKind::Instance { dict: _ } => format!("<instance>"),
      PyObjectKind::Code { code: _ } => format!("<code>"),
      PyObjectKind::Function { code: _, scope: _ } => format!("<func>"),
      PyObjectKind::BoundMethod { .. } => format!("<bound-method>"),
      PyObjectKind::RustFunction { function: _ } => format!("<rustfunc>"),
      PyObjectKind::Module { ref name, dict: _ } => {
        format!("<module '{}'>", name)
      }
      PyObjectKind::Scope { ref scope } => format!("<scope '{:?}'>", scope),
      PyObjectKind::NameError { ref name } => format!("NameError: {:?}", name),
      PyObjectKind::Slice {
        ref start,
        ref stop,
        ref step,
      } => format!("<slice '{:?}:{:?}:{:?}'>", start, stop, step),
      PyObjectKind::Iterator {
        ref position,
        ref iterated_obj,
      } => format!(
        "<iter pos {} in {}>",
        position,
        iterated_obj.borrow_mut().str()
      ),
    }
  }

  // Implement iterator protocol:
  pub fn nxt(&mut self) -> Option<PyObjectRef> {
    match self.kind {
      PyObjectKind::Iterator {
        ref mut position,
        iterated_obj: ref iterated_obj_ref,
      } => {
        let iterated_obj = &*iterated_obj_ref.borrow_mut();
        match iterated_obj.kind {
          PyObjectKind::List { ref elements } => {
            if *position < elements.len() {
              let obj_ref = elements[*position].clone();
              *position += 1;
              Some(obj_ref)
            } else {
              None
            }
          }
          _ => {
            panic!("NOT IMPL");
          }
        }
      }
      _ => {
        panic!("NOT IMPL");
      }
    }
  }

  // Move this object into a reference object, transferring ownership.
  pub fn into_ref(self) -> PyObjectRef {
    Rc::new(RefCell::new(self))
  }
}

impl fmt::Debug for PyObject {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[PyObject {:?}]", self.kind)
  }
}

pub trait IdProtocol {
  fn get_id(&self) -> usize;
}

impl IdProtocol for PyObjectRef {
  fn get_id(&self) -> usize {
    self.as_ptr() as usize
  }
}

pub trait TypeProtocol {
  fn typ(&self) -> PyObjectRef;
}

impl TypeProtocol for PyObjectRef {
  fn typ(&self) -> PyObjectRef {
    match self.borrow().typ {
      Some(ref typ) => typ.clone(),
      None => panic!("Object doesn't have a type!"),
    }
  }
}

pub trait ParentProtocol {
  fn has_parent(&self) -> bool;
  fn get_parent(&self) -> PyObjectRef;
}

impl ParentProtocol for PyObjectRef {
  fn has_parent(&self) -> bool {
    match self.borrow().kind {
      PyObjectKind::Scope { ref scope } => match scope.parent {
        Some(_) => true,
        None => false,
      },
      _ => panic!("Only scopes have parent (not {:?}", self),
    }
  }

  fn get_parent(&self) -> PyObjectRef {
    match self.borrow().kind {
      PyObjectKind::Scope { ref scope } => match scope.parent {
        Some(ref value) => value.clone(),
        None => panic!("OMG"),
      },
      _ => panic!("TODO"),
    }
  }
}

pub trait AttributeProtocol {
  fn get_attr(&self, attr_name: &String) -> PyObjectRef;
  fn set_attr(&self, attr_name: &str, value: PyObjectRef);
  fn has_attr(&self, attr_name: &String) -> bool;
}

fn class_get_item(
  class: &PyObjectRef,
  attr_name: &String,
) -> Option<PyObjectRef> {
  let class = class.borrow();
  match class.kind {
    PyObjectKind::Class { ref dict, .. } => {
      if dict.contains_key(attr_name) {
        return Some(dict.get_item(attr_name));
      }
      None
    }
    _ => panic!("Only classes should be in MRO!"),
  }
}

fn class_has_item(class: &PyObjectRef, attr_name: &String) -> bool {
  let class = class.borrow();
  match class.kind {
    PyObjectKind::Class { ref dict, .. } => dict.contains_key(attr_name),
    _ => panic!("Only classes should be in MRO!"),
  }
}

impl AttributeProtocol for PyObjectRef {
  fn get_attr(&self, attr_name: &String) -> PyObjectRef {
    let obj = self.borrow();
    match obj.kind {
      PyObjectKind::Module { ref dict, .. } => dict.get_item(attr_name),
      PyObjectKind::Class { ref mro, .. } => {
        if let Some(item) = class_get_item(self, attr_name) {
          return item;
        }
        for ref class in mro {
          if let Some(item) = class_get_item(class, attr_name) {
            return item;
          }
        }
        panic!("MRO search failed: {:?} {}", obj, attr_name);
      }
      PyObjectKind::Instance { ref dict } => dict.get_item(attr_name),
      ref kind => unimplemented!("load_attr unimplemented for: {:?}", kind),
    }
  }

  fn has_attr(&self, attr_name: &String) -> bool {
    let obj = self.borrow();
    match obj.kind {
      PyObjectKind::Module { name: _, ref dict } => {
        dict.contains_key(attr_name)
      }
      PyObjectKind::Class { ref mro, .. } => {
        class_has_item(self, attr_name)
          || mro.into_iter().any(|d| class_has_item(d, attr_name))
      }
      PyObjectKind::Instance { ref dict } => dict.contains_key(attr_name),
      _ => false,
    }
  }

  fn set_attr(&self, attr_name: &str, value: PyObjectRef) {
    match self.borrow().kind {
      PyObjectKind::Instance { ref dict } => {
        dict.set_item(&String::from(attr_name), value)
      }
      PyObjectKind::Class {
        name: _,
        ref dict,
        mro: _,
      } => dict.set_item(&String::from(attr_name), value),
      ref kind => unimplemented!("set_attr unimplemented for: {:?}", kind),
    };
  }
}

pub trait DictProtocol {
  fn contains_key(&self, k: &String) -> bool;
  fn get_item(&self, k: &String) -> PyObjectRef;
  fn set_item(&self, k: &String, v: PyObjectRef);
}

impl DictProtocol for PyObjectRef {
  fn contains_key(&self, k: &String) -> bool {
    match self.borrow().kind {
      PyObjectKind::Dict { ref elements } => elements.contains_key(k),
      PyObjectKind::Module { name: _, ref dict } => dict.contains_key(k),
      PyObjectKind::Scope { ref scope } => scope.locals.contains_key(k),
      ref kind => unimplemented!("TODO {:?}", kind),
    }
  }

  fn get_item(&self, k: &String) -> PyObjectRef {
    match self.borrow().kind {
      PyObjectKind::Dict { ref elements } => elements[k].clone(),
      PyObjectKind::Module { name: _, ref dict } => dict.get_item(k),
      PyObjectKind::Scope { ref scope } => scope.locals.get_item(k),
      _ => panic!("TODO"),
    }
  }

  fn set_item(&self, k: &String, v: PyObjectRef) {
    match self.borrow_mut().kind {
      PyObjectKind::Dict {
        elements: ref mut el,
      } => {
        el.insert(k.to_string(), v);
      }
      PyObjectKind::Module {
        name: _,
        ref mut dict,
      } => dict.set_item(k, v),
      PyObjectKind::Scope { ref mut scope } => {
        scope.locals.set_item(k, v);
      }
      _ => panic!("TODO"),
    };
  }
}

pub trait ToRust {
  fn to_vec(&self) -> Option<Vec<PyObjectRef>>;
  fn to_str(&self) -> Option<String>;
}

impl ToRust for PyObjectRef {
  fn to_vec(&self) -> Option<Vec<PyObjectRef>> {
    match self.borrow().kind {
      PyObjectKind::Tuple { ref elements } => Some(elements.clone()),
      PyObjectKind::List { ref elements } => Some(elements.clone()),
      _ => None,
    }
  }

  fn to_str(&self) -> Option<String> {
    Some(self.borrow().str())
  }
}

#[derive(Debug, Default, Clone)]
pub struct PyFuncArgs {
  pub args: Vec<PyObjectRef>,
  // TODO: add kwargs here
}

impl PyFuncArgs {
  pub fn insert(&self, item: PyObjectRef) -> PyFuncArgs {
    let mut args = PyFuncArgs {
      args: self.args.clone(),
    };
    args.args.insert(0, item);
    return args;
  }
  pub fn shift(&mut self) -> PyObjectRef {
    self.args.remove(0)
  }
}

type RustPyFunc = fn(vm: &mut VirtualMachine, PyFuncArgs) -> PyResult;

pub enum PyObjectKind {
  String {
    value: String,
  },
  Integer {
    value: i32,
  },
  Float {
    value: f64,
  },
  Boolean {
    value: bool,
  },
  List {
    elements: Vec<PyObjectRef>,
  },
  Tuple {
    elements: Vec<PyObjectRef>,
  },
  Dict {
    elements: HashMap<String, PyObjectRef>,
  },
  Iterator {
    position: usize,
    iterated_obj: PyObjectRef,
  },
  Slice {
    start: Option<i32>,
    stop: Option<i32>,
    step: Option<i32>,
  },
  NameError {
    // TODO: improve python object and type system
    name: String,
  },
  Code {
    code: bytecode::CodeObject,
  },
  Function {
    code: PyObjectRef,
    scope: PyObjectRef,
  },
  BoundMethod {
    function: PyObjectRef,
    object: PyObjectRef,
  },
  Scope {
    scope: Scope,
  },
  Module {
    name: String,
    dict: PyObjectRef,
  },
  PyNone,
  Class {
    name: String,
    dict: PyObjectRef,
    mro: Vec<PyObjectRef>,
  },
  Instance {
    dict: PyObjectRef,
  },
  RustFunction {
    function: RustPyFunc,
  },
}

impl fmt::Debug for PyObjectKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &PyObjectKind::String { ref value } => write!(f, "str \"{}\"", value),
      &PyObjectKind::Integer { ref value } => write!(f, "int {}", value),
      &PyObjectKind::Float { ref value } => write!(f, "float {}", value),
      &PyObjectKind::Boolean { ref value } => write!(f, "boolean {}", value),
      &PyObjectKind::List { elements: _ } => write!(f, "list"),
      &PyObjectKind::Tuple { elements: _ } => write!(f, "tuple"),
      &PyObjectKind::Dict { elements: _ } => write!(f, "dict"),
      &PyObjectKind::Iterator {
        position: _,
        iterated_obj: _,
      } => write!(f, "iterator"),
      &PyObjectKind::Slice {
        start: _,
        stop: _,
        step: _,
      } => write!(f, "slice"),
      &PyObjectKind::NameError { name: _ } => write!(f, "NameError"),
      &PyObjectKind::Code { ref code } => write!(f, "code: {:?}", code),
      &PyObjectKind::Function { code: _, scope: _ } => write!(f, "function"),
      &PyObjectKind::BoundMethod {
        ref function,
        ref object,
      } => write!(f, "bound-method: {:?} of {:?}", function, object),
      &PyObjectKind::Module { name: _, dict: _ } => write!(f, "module"),
      &PyObjectKind::Scope { scope: _ } => write!(f, "scope"),
      &PyObjectKind::PyNone => write!(f, "None"),
      &PyObjectKind::Class {
        ref name,
        dict: _,
        mro: _,
      } => write!(f, "class {:?}", name),
      &PyObjectKind::Instance { dict: _ } => write!(f, "instance"),
      &PyObjectKind::RustFunction { function: _ } => write!(f, "rust function"),
    }
  }
}

impl<'a> Add<&'a PyObject> for &'a PyObject {
  type Output = PyObjectKind;

  fn add(self, rhs: &'a PyObject) -> Self::Output {
    match self.kind {
      PyObjectKind::Integer { value: ref value1 } => match &rhs.kind {
        PyObjectKind::Integer { value: ref value2 } => PyObjectKind::Integer {
          value: value1 + value2,
        },
        PyObjectKind::Float { value: ref value2 } => PyObjectKind::Float {
          value: (*value1 as f64) + value2,
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      PyObjectKind::Float { value: ref value1 } => match &rhs.kind {
        PyObjectKind::Float { value: ref value2 } => PyObjectKind::Float {
          value: value1 + value2,
        },
        PyObjectKind::Integer { value: ref value2 } => PyObjectKind::Float {
          value: value1 + (*value2 as f64),
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      PyObjectKind::String { value: ref value1 } => match rhs.kind {
        PyObjectKind::String { value: ref value2 } => PyObjectKind::String {
          value: format!("{}{}", value1, value2),
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      PyObjectKind::List { elements: ref e1 } => match rhs.kind {
        PyObjectKind::List { elements: ref e2 } => PyObjectKind::List {
          elements: e1.iter().chain(e2.iter()).map(|e| e.clone()).collect(),
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      _ => {
        // TODO: Lookup __add__ method in dictionary?
        panic!("NOT IMPL");
      }
    }
  }
}

impl<'a> Sub<&'a PyObject> for &'a PyObject {
  type Output = PyObjectKind;

  fn sub(self, rhs: &'a PyObject) -> Self::Output {
    match self.kind {
      PyObjectKind::Integer { value: value1 } => match rhs.kind {
        PyObjectKind::Integer { value: value2 } => PyObjectKind::Integer {
          value: value1 - value2,
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      _ => {
        panic!("NOT IMPL");
      }
    }
  }
}

impl<'a> Mul<&'a PyObject> for &'a PyObject {
  type Output = PyObjectKind;

  fn mul(self, rhs: &'a PyObject) -> Self::Output {
    match self.kind {
      PyObjectKind::Integer { value: value1 } => match rhs.kind {
        PyObjectKind::Integer { value: value2 } => PyObjectKind::Integer {
          value: value1 * value2,
        },
        _ => {
          panic!("NOT IMPL");
        }
      },
      PyObjectKind::String { value: ref value1 } => match rhs.kind {
        PyObjectKind::Integer { value: value2 } => {
          let mut result = String::new();
          for _x in 0..value2 {
            result.push_str(value1.as_str());
          }
          PyObjectKind::String { value: result }
        }
        _ => {
          panic!("NOT IMPL");
        }
      },
      _ => {
        panic!("NOT IMPL");
      }
    }
  }
}

impl<'a> Div<&'a PyObject> for &'a PyObject {
  type Output = PyObjectKind;

  fn div(self, rhs: &'a PyObject) -> Self::Output {
    match (&self.kind, &rhs.kind) {
      (
        PyObjectKind::Integer { value: value1 },
        PyObjectKind::Integer { value: value2 },
      ) => PyObjectKind::Integer {
        value: value1 / value2,
      },
      _ => {
        panic!("NOT IMPL");
      }
    }
  }
}

impl<'a> Rem<&'a PyObject> for &'a PyObject {
  type Output = PyObjectKind;

  fn rem(self, rhs: &'a PyObject) -> Self::Output {
    match (&self.kind, &rhs.kind) {
      (
        PyObjectKind::Integer { value: value1 },
        PyObjectKind::Integer { value: value2 },
      ) => PyObjectKind::Integer {
        value: value1 % value2,
      },
      (kind1, kind2) => {
        unimplemented!("% not implemented for kinds: {:?} {:?}", kind1, kind2);
      }
    }
  }
}

// impl<'a> PartialEq<&'a PyObject> for &'a PyObject {
impl PartialEq for PyObject {
  fn eq(&self, other: &PyObject) -> bool {
    match (&self.kind, &other.kind) {
      (
        PyObjectKind::Integer { value: ref v1i },
        PyObjectKind::Integer { value: ref v2i },
      ) => v2i == v1i,
      (
        PyObjectKind::Float { value: ref v1i },
        PyObjectKind::Float { value: ref v2i },
      ) => v2i == v1i,
      (
        PyObjectKind::String { value: ref v1i },
        PyObjectKind::String { value: ref v2i },
      ) => *v2i == *v1i,
      /*
      (&NativeType::Float(ref v1f), &NativeType::Float(ref v2f)) => {
          curr_frame.stack.push(Rc::new(NativeType::Boolean(v2f == v1f)));
      },
      */
      (
        PyObjectKind::List { elements: ref l1 },
        PyObjectKind::List { elements: ref l2 },
      )
      | (
        PyObjectKind::Tuple { elements: ref l1 },
        PyObjectKind::Tuple { elements: ref l2 },
      ) => {
        if l1.len() == l2.len() {
          Iterator::zip(l1.iter(), l2.iter()).all(|elem| elem.0 == elem.1)
        } else {
          false
        }
      }
      _ => panic!(
        "TypeError in COMPARE_OP: can't compare {:?} with {:?}",
        self, other
      ),
    }
  }
}

impl Eq for PyObject {}

impl PartialOrd for PyObject {
  fn partial_cmp(&self, other: &PyObject) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for PyObject {
  fn cmp(&self, other: &PyObject) -> Ordering {
    match (&self.kind, &other.kind) {
      (
        PyObjectKind::Integer { value: v1 },
        PyObjectKind::Integer { value: ref v2 },
      ) => v1.cmp(v2),
      _ => panic!("Not impl"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{PyContext, PyObjectKind};

  #[test]
  fn test_add_py_integers() {
    let ctx = PyContext::new();
    let a = ctx.new_int(33);
    let b = ctx.new_int(12);
    let c = &*a.borrow() + &*b.borrow();
    match c {
      PyObjectKind::Integer { value } => assert_eq!(value, 45),
      _ => assert!(false),
    }
  }

  #[test]
  fn test_multiply_str() {
    let ctx = PyContext::new();
    let a = ctx.new_str(String::from("Hello "));
    let b = ctx.new_int(4);
    let c = &*a.borrow() * &*b.borrow();
    match c {
      PyObjectKind::String { value } => {
        assert_eq!(value, String::from("Hello Hello Hello Hello "))
      }
      _ => assert!(false),
    }
  }

  #[test]
  fn test_type_type() {
    // TODO: Write this test
    PyContext::new();
  }
}
