use super::pyobject::{PyObject, PyObjectKind, PyObjectRef, PyResult};
use super::vm::VirtualMachine;
use std::marker::Sized;

pub trait PySliceableSequence {
  fn do_slice(&self, start: usize, stop: usize) -> Self;
  fn do_stepped_slice(&self, start: usize, stop: usize, step: usize) -> Self;
  fn len(&self) -> usize;
  fn get_pos(&self, p: i32) -> usize {
    if p < 0 {
      self.len() - ((-p) as usize)
    } else if p as usize > self.len() {
      self.len()
    } else {
      p as usize
    }
  }
  fn get_slice_items(&self, slice: &PyObjectRef) -> Self
  where
    Self: Sized,
  {
    match &(slice.borrow()).kind {
      PyObjectKind::Slice { start, stop, step } => {
        let start = match start {
          &Some(start) => self.get_pos(start),
          &None => 0,
        };
        let stop = match stop {
          &Some(stop) => self.get_pos(stop),
          &None => self.len() as usize,
        };
        match step {
          &None | &Some(1) => self.do_slice(start, stop),
          &Some(num) => {
            if num < 0 {
              unimplemented!("negative step indexing not yet supported")
            };
            self.do_stepped_slice(start, stop, num as usize)
          }
        }
      }
      kind => panic!("get_slice_items called with non-slice: {:?}", kind),
    }
  }
}

impl PySliceableSequence for Vec<PyObjectRef> {
  fn do_slice(&self, start: usize, stop: usize) -> Self {
    self[start..stop].to_vec()
  }
  fn do_stepped_slice(&self, start: usize, stop: usize, step: usize) -> Self {
    self[start..stop].iter().step_by(step).cloned().collect()
  }
  fn len(&self) -> usize {
    self.len()
  }
}

pub fn get_item(
  vm: &mut VirtualMachine,
  sequence: &PyObjectRef,
  elements: &Vec<PyObjectRef>,
  subscript: PyObjectRef,
) -> PyResult {
  match &(subscript.borrow()).kind {
    PyObjectKind::Integer { value } => {
      let pos_index = elements.get_pos(*value);
      if pos_index < elements.len() {
        let obj = elements[pos_index].clone();
        Ok(obj)
      } else {
        Err(vm.new_exception("Index out of bounds!".to_string()))
      }
    }
    PyObjectKind::Slice {
      start: _,
      stop: _,
      step: _,
    } => Ok(PyObject::new(
      match &(sequence.borrow()).kind {
        PyObjectKind::Tuple { elements: _ } => PyObjectKind::Tuple {
          elements: elements.get_slice_items(&subscript),
        },
        PyObjectKind::List { elements: _ } => PyObjectKind::List {
          elements: elements.get_slice_items(&subscript),
        },
        ref kind => {
          panic!("sequence get_item called for non-sequence: {:?}", kind)
        }
      },
      vm.get_type(),
    )),
    _ => Err(vm.new_exception(format!(
      "TypeError: indexing type {:?} with index {:?} is not supported (yet?)",
      sequence, subscript
    ))),
  }
}
