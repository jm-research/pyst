use std::cell::RefMut;
use std::collections::hash_map::HashMap;
use std::ops::Deref;

use super::builtins;
use super::bytecode;
use super::frame::{copy_code, Block, Frame};
use super::import::import;
use super::objlist;
use super::objobject;
use super::objstr;
use super::objtype;
use super::pyobject::{
  AttributeProtocol, DictProtocol, IdProtocol, ParentProtocol, PyContext,
  PyFuncArgs, PyObject, PyObjectKind, PyObjectRef, PyResult,
};
use super::sysmodule;

pub struct VirtualMachine {
  frames: Vec<Frame>,
  builtins: PyObjectRef,
  pub sys_module: PyObjectRef,
  ctx: PyContext,
}

impl VirtualMachine {
  pub fn run_code_obj(
    &mut self,
    code: PyObjectRef,
    scope: PyObjectRef,
  ) -> PyResult {
    let frame = Frame::new(code, scope);
    self.run_frame(frame)
  }

  pub fn new_str(&self, s: String) -> PyObjectRef {
    self.ctx.new_str(s)
  }

  pub fn new_bool(&self, b: bool) -> PyObjectRef {
    self.ctx.new_bool(b)
  }

  pub fn new_dict(&self) -> PyObjectRef {
    self.ctx.new_dict()
  }

  pub fn new_exception(&self, msg: String) -> PyObjectRef {
    self.new_str(msg)
  }

  pub fn new_scope(&mut self) -> PyObjectRef {
    let parent_scope = self.current_frame().locals.clone();
    self.ctx.new_scope(Some(parent_scope))
  }

  pub fn get_none(&self) -> PyObjectRef {
    self.ctx.none_type.clone()
  }

  pub fn new_bound_method(
    &self,
    function: PyObjectRef,
    object: PyObjectRef,
  ) -> PyObjectRef {
    self.ctx.new_bound_method(function, object)
  }

  pub fn get_type(&self) -> PyObjectRef {
    self.ctx.type_type.clone()
  }

  pub fn get_object(&self) -> PyObjectRef {
    self.ctx.object_type.clone()
  }

  pub fn get_locals(&self) -> PyObjectRef {
    let scope = &self.frames.last().unwrap().locals;
    scope.clone()
    /*
    match (*scope).kind {
        PyObjectKind::Scope { scope } => { scope.locals.clone() },
        _ => { panic!("Should be scope") },
    } // .clone()
    */
  }

  pub fn context(&self) -> &PyContext {
    &self.ctx
  }

  pub fn new() -> VirtualMachine {
    let ctx = PyContext::new();
    let builtins = builtins::make_module(&ctx);
    let sysmod = sysmodule::mk_module(&ctx);
    VirtualMachine {
      frames: vec![],
      builtins: builtins,
      sys_module: sysmod,
      ctx: ctx,
    }
  }

  pub fn get_builtin_scope(&mut self) -> PyObjectRef {
    let a2 = &*self.builtins.borrow();
    match a2.kind {
      PyObjectKind::Module { name: _, ref dict } => dict.clone(),
      _ => {
        panic!("OMG");
      }
    }
  }

  // Container of the virtual machine state:
  pub fn to_str(&mut self, obj: PyObjectRef) -> String {
    obj.borrow().str()
  }

  fn current_frame(&mut self) -> &mut Frame {
    self.frames.last_mut().unwrap()
  }

  fn pop_frame(&mut self) -> Frame {
    self.frames.pop().unwrap()
  }

  fn push_block(&mut self, block: Block) {
    self.current_frame().push_block(block);
  }

  fn pop_block(&mut self) -> Option<Block> {
    self.current_frame().pop_block()
  }

  fn last_block(&mut self) -> &Block {
    self.current_frame().last_block()
  }

  fn unwind_loop(&mut self) -> Block {
    loop {
      let block = self.pop_block();
      match block {
        Some(Block::Loop { start: _, end: __ }) => break block.unwrap(),
        Some(Block::TryExcept {}) => {}
        None => panic!("No block to break / continue"),
      }
    }
  }

  fn unwind_exception(&mut self, exc: PyObjectRef) -> Option<PyObjectRef> {
    // unwind block stack on exception and find any handlers:
    loop {
      let block = self.pop_block();
      match block {
        Some(Block::TryExcept {}) => {
          // Exception handled?
          // TODO: how do we know if the exception is handled?
          let is_handled = true;
          if is_handled {
            return None;
          }
        }
        Some(_) => {}
        None => break,
      }
    }
    Some(exc)
  }

  fn push_value(&mut self, obj: PyObjectRef) {
    self.current_frame().push_value(obj);
  }

  fn pop_value(&mut self) -> PyObjectRef {
    self.current_frame().pop_value()
  }

  fn pop_multiple(&mut self, count: usize) -> Vec<PyObjectRef> {
    self.current_frame().pop_multiple(count)
  }

  fn last_value(&mut self) -> PyObjectRef {
    self.current_frame().last_value()
  }

  fn store_name(&mut self, name: &String) -> Option<PyResult> {
    let obj = self.pop_value();
    self.current_frame().locals.set_item(name, obj);
    None
  }

  fn load_name(&mut self, name: &String) -> Option<PyResult> {
    // Lookup name in scope and put it onto the stack!
    let mut scope = self.current_frame().locals.clone();
    loop {
      if scope.contains_key(name) {
        let obj = scope.get_item(name);
        self.push_value(obj);
        break None;
      } else if scope.has_parent() {
        scope = scope.get_parent();
      } else {
        let name_error = PyObject::new(
          PyObjectKind::NameError {
            name: name.to_string(),
          },
          self.get_type(),
        );
        break Some(Err(name_error));
      }
    }
  }

  fn run_frame(&mut self, frame: Frame) -> PyResult {
    self.frames.push(frame);

    // Execute until return or exception:
    let value = loop {
      let result = self.execute_instruction();
      match result {
        None => {}
        Some(Ok(value)) => {
          break Ok(value);
        }
        Some(Err(exception)) => {
          // unwind block stack on exception and find any handlers.
          match self.unwind_exception(exception) {
            None => {}
            Some(exception) => {
              let _traceback = self
                .get_attribute(exception.clone(), &"__traceback__".to_string());
              // TODO: append line number to traceback?
              // traceback.append();
              break Err(exception);
            }
          }
        }
      }
    };

    self.pop_frame();
    value
  }

  fn subscript(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    // Subscript implementation: a[b]
    let a2 = &*a.borrow();
    match &a2.kind {
      PyObjectKind::String { ref value } => objstr::subscript(self, value, b),
      PyObjectKind::List { ref elements }
      | PyObjectKind::Tuple { ref elements } => {
        super::objsequence::get_item(self, &a, elements, b)
      }
      _ => Err(self.new_exception(format!(
        "TypeError: indexing type {:?} with index {:?} is not supported (yet?)",
        a, b
      ))),
    }
  }

  fn execute_store_subscript(&mut self) -> Option<PyResult> {
    let idx = self.pop_value();
    let obj = self.pop_value();
    let value = self.pop_value();
    let a2 = &mut *obj.borrow_mut();
    let result = match &mut a2.kind {
          PyObjectKind::List { ref mut elements } => {
              objlist::set_item(self, elements, idx, value)
          }
          _ => Err(self.new_exception(format!(
              "TypeError: __setitem__ assign type {:?} with index {:?} is not supported (yet?)",
              obj, idx
          ))),
      };

    match result {
      Ok(_) => None,
      Err(value) => Some(Err(value)),
    }
  }

  fn _sub(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    Ok(PyObject::new(a2 - b2, self.get_type()))
  }

  fn _add(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    Ok(PyObject::new(a2 + b2, self.get_type()))
  }

  fn _mul(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    Ok(PyObject::new(a2 * b2, self.get_type()))
  }

  fn _div(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    Ok(PyObject::new(a2 / b2, self.get_type()))
  }

  fn _pow(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    match (&a2.kind, &b2.kind) {
      (
        &PyObjectKind::Integer { value: ref v1 },
        &PyObjectKind::Integer { value: ref v2 },
      ) => Ok(self.ctx.new_int(v1.pow(*v2 as u32))),
      (
        &PyObjectKind::Float { value: ref v1 },
        &PyObjectKind::Integer { value: ref v2 },
      ) => Ok(self.ctx.new_float(v1.powf(*v2 as f64))),
      (
        &PyObjectKind::Integer { value: ref v1 },
        &PyObjectKind::Float { value: ref v2 },
      ) => Ok(self.ctx.new_float((*v1 as f64).powf(*v2))),
      (
        &PyObjectKind::Float { value: ref v1 },
        &PyObjectKind::Float { value: ref v2 },
      ) => Ok(self.ctx.new_float(v1.powf(*v2))),
      _ => panic!("Not impl"),
    }
  }

  fn _modulo(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    Ok(PyObject::new(a2 % b2, self.get_type()))
  }

  fn execute_binop(
    &mut self,
    op: &bytecode::BinaryOperator,
  ) -> Option<PyResult> {
    let b_ref = self.pop_value();
    let a_ref = self.pop_value();
    // TODO: if the left hand side provides __add__, invoke that function.
    //
    let result = match op {
      &bytecode::BinaryOperator::Subtract => self._sub(a_ref, b_ref),
      &bytecode::BinaryOperator::Add => self._add(a_ref, b_ref),
      &bytecode::BinaryOperator::Multiply => self._mul(a_ref, b_ref),
      &bytecode::BinaryOperator::Power => self._pow(a_ref, b_ref),
      &bytecode::BinaryOperator::Divide => self._div(a_ref, b_ref),
      &bytecode::BinaryOperator::Subscript => self.subscript(a_ref, b_ref),
      &bytecode::BinaryOperator::Modulo => self._modulo(a_ref, b_ref),
      _ => panic!("NOT IMPL {:?}", op),
    };
    match result {
      Ok(value) => {
        self.push_value(value);
        None
      }
      Err(value) => Some(Err(value)),
    }
  }

  fn execute_unop(&mut self, op: &bytecode::UnaryOperator) -> Option<PyResult> {
    let a_ref = self.pop_value();
    let a = &*a_ref.borrow();
    let result = match op {
      &bytecode::UnaryOperator::Minus => {
        // TODO:
        // self.invoke('__neg__'
        match a.kind {
          PyObjectKind::Integer { value: ref value1 } => {
            Ok(self.ctx.new_int(-*value1))
          }
          _ => panic!("Not impl {:?}", a),
        }
      }
      &bytecode::UnaryOperator::Not => {
        // TODO:
        // self.invoke('__neg__'
        match a.kind {
          PyObjectKind::Boolean { value: ref value1 } => {
            Ok(self.ctx.new_bool(!*value1))
          }
          _ => panic!("Not impl {:?}", a),
        }
      }
      _ => panic!("Not impl {:?}", op),
    };
    match result {
      Ok(value) => {
        self.push_value(value);
        None
      }
      Err(value) => Some(Err(value)),
    }
  }

  fn _eq(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 == b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _ne(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 != b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _lt(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 < b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _le(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 <= b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _gt(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 > b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _ge(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    let b2 = &*b.borrow();
    let a2 = &*a.borrow();
    let result_bool = a2 >= b2;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _id(&mut self, a: PyObjectRef) -> usize {
    a.get_id()
  }

  fn _is(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    // Pointer equal:
    let id_a = self._id(a);
    let id_b = self._id(b);
    let result_bool = id_a == id_b;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn _is_not(&mut self, a: PyObjectRef, b: PyObjectRef) -> PyResult {
    // Pointer equal:
    let id_a = self._id(a);
    let id_b = self._id(b);
    let result_bool = id_a != id_b;
    let result = self.ctx.new_bool(result_bool);
    Ok(result)
  }

  fn execute_compare(
    &mut self,
    op: &bytecode::ComparisonOperator,
  ) -> Option<PyResult> {
    let b = self.pop_value();
    let a = self.pop_value();
    let result = match op {
      &bytecode::ComparisonOperator::Equal => self._eq(a, b),
      &bytecode::ComparisonOperator::NotEqual => self._ne(a, b),
      &bytecode::ComparisonOperator::Less => self._lt(a, b),
      &bytecode::ComparisonOperator::LessOrEqual => self._le(a, b),
      &bytecode::ComparisonOperator::Greater => self._gt(a, b),
      &bytecode::ComparisonOperator::GreaterOrEqual => self._ge(a, b),
      &bytecode::ComparisonOperator::Is => self._is(a, b),
      &bytecode::ComparisonOperator::IsNot => self._is_not(a, b),
      _ => panic!("NOT IMPL {:?}", op),
    };
    match result {
      Ok(value) => {
        self.push_value(value);
        None
      }
      Err(value) => Some(Err(value)),
    }
  }

  pub fn invoke(
    &mut self,
    func_ref: PyObjectRef,
    args: PyFuncArgs,
  ) -> PyResult {
    trace!("Invoke: {:?} {:?}", func_ref, args);
    match func_ref.borrow().kind {
      PyObjectKind::RustFunction { function } => function(self, args),
      PyObjectKind::Function {
        ref code,
        ref scope,
      } => {
        let scope = self.ctx.new_scope(Some(scope.clone()));
        let code_object = copy_code(code.clone());
        for (name, value) in code_object.arg_names.iter().zip(args.args) {
          scope.set_item(name, value);
        }
        let frame = Frame::new(code.clone(), scope);
        self.run_frame(frame)
      }
      PyObjectKind::Class {
        name: _,
        dict: _,
        mro: _,
      } => objtype::call(self, func_ref.clone(), args),
      PyObjectKind::BoundMethod {
        ref function,
        ref object,
      } => self.invoke(function.clone(), args.insert(object.clone())),
      PyObjectKind::Instance { .. } => {
        objobject::call(self, args.insert(func_ref.clone()))
      }
      ref kind => {
        unimplemented!("invoke unimplemented for: {:?}", kind);
      }
    }
  }

  fn import(
    &mut self,
    module: &String,
    symbol: &Option<String>,
  ) -> Option<PyResult> {
    let obj = match import(self, module, symbol) {
      Ok(value) => value,
      Err(value) => return Some(Err(value)),
    };

    // Push module on stack:
    self.push_value(obj);
    None
  }

  pub fn get_attribute(
    &mut self,
    obj: PyObjectRef,
    attr_name: &String,
  ) -> PyResult {
    objtype::get_attribute(self, obj.clone(), attr_name)
  }

  fn load_attr(&mut self, attr_name: &String) -> Option<PyResult> {
    let parent = self.pop_value();
    match self.get_attribute(parent, attr_name) {
      Ok(obj) => {
        self.push_value(obj);
        None
      }
      Err(err) => Some(Err(err)),
    }
  }

  fn store_attr(&mut self, attr_name: &String) -> Option<PyResult> {
    let parent = self.pop_value();
    let value = self.pop_value();
    parent.set_attr(attr_name, value);
    None
  }

  // Execute a single instruction:
  fn execute_instruction(&mut self) -> Option<PyResult> {
    let instruction = self.current_frame().fetch_instruction();
    {
      trace!("=======");
      /* TODO:
      for frame in self.frames.iter() {
          trace!("  {:?}", frame);
      }
      */
      trace!("  {:?}", self.current_frame());
      trace!("  Executing op code: {:?}", instruction);
      trace!("=======");
    }
    match &instruction {
      bytecode::Instruction::LoadConst { ref value } => {
        let obj = match value {
          &bytecode::Constant::Integer { ref value } => {
            self.ctx.new_int(*value)
          }
          &bytecode::Constant::Float { ref value } => {
            self.ctx.new_float(*value)
          }
          &bytecode::Constant::String { ref value } => {
            self.new_str(value.clone())
          }
          &bytecode::Constant::Boolean { ref value } => {
            self.new_bool(value.clone())
          }
          &bytecode::Constant::Code { ref code } => PyObject::new(
            PyObjectKind::Code { code: code.clone() },
            self.get_type(),
          ),
          &bytecode::Constant::None => self.ctx.none_type.clone(),
        };
        self.push_value(obj);
        None
      }
      bytecode::Instruction::Import {
        ref name,
        ref symbol,
      } => self.import(name, symbol),
      bytecode::Instruction::LoadName { ref name } => self.load_name(name),
      bytecode::Instruction::StoreName { ref name } => {
        // take top of stack and assign in scope:
        self.store_name(name)
      }
      bytecode::Instruction::StoreSubscript => self.execute_store_subscript(),
      bytecode::Instruction::Pop => {
        // Pop value from stack and ignore.
        self.pop_value();
        None
      }
      bytecode::Instruction::BuildList { size } => {
        let elements = self.pop_multiple(*size);
        let list_obj = self.context().new_list(elements);
        self.push_value(list_obj);
        None
      }
      bytecode::Instruction::BuildTuple { size } => {
        let elements = self.pop_multiple(*size);
        let list_obj = self.context().new_tuple(elements);
        self.push_value(list_obj);
        None
      }
      bytecode::Instruction::BuildMap { size } => {
        let mut elements = HashMap::new();
        for _x in 0..*size {
          let obj = self.pop_value();
          // XXX: Currently, we only support String keys, so we have to unwrap the
          // PyObject (and ensure it is a String).
          let key_pyobj = self.pop_value();
          let key = match key_pyobj.borrow().kind {
            PyObjectKind::String { ref value } => value.clone(),
            ref kind => unimplemented!(
              "Only strings can be used as dict keys, we saw: {:?}",
              kind
            ),
          };
          elements.insert(key, obj);
        }
        let map_obj = PyObject::new(
          PyObjectKind::Dict { elements: elements },
          self.get_type(),
        );
        self.push_value(map_obj);
        None
      }
      bytecode::Instruction::BuildSlice { size } => {
        assert!(*size == 2 || *size == 3);
        let elements = self.pop_multiple(*size);

        let out: Vec<Option<i32>> = elements
          .into_iter()
          .map(|x| match x.borrow().kind {
            PyObjectKind::Integer { value } => Some(value),
            PyObjectKind::PyNone => None,
            _ => {
              panic!("Expect Int or None as BUILD_SLICE arguments, got {:?}", x)
            }
          })
          .collect();

        let start = out[0];
        let stop = out[1];
        let step = if out.len() == 3 { out[2] } else { None };

        let obj = PyObject::new(
          PyObjectKind::Slice { start, stop, step },
          self.ctx.type_type.clone(),
        );
        self.push_value(obj);
        None
      }
      bytecode::Instruction::BinaryOperation { ref op } => {
        self.execute_binop(op)
      }
      bytecode::Instruction::LoadAttr { ref name } => self.load_attr(name),
      bytecode::Instruction::StoreAttr { ref name } => self.store_attr(name),
      bytecode::Instruction::UnaryOperation { ref op } => self.execute_unop(op),
      bytecode::Instruction::CompareOperation { ref op } => {
        self.execute_compare(op)
      }
      bytecode::Instruction::ReturnValue => {
        let value = self.pop_value();
        Some(Ok(value))
      }
      bytecode::Instruction::SetupLoop { start, end } => {
        self.push_block(Block::Loop {
          start: *start,
          end: *end,
        });
        None
      }
      bytecode::Instruction::PopBlock => {
        self.pop_block();
        None
      }
      bytecode::Instruction::GetIter => {
        let iterated_obj = self.pop_value();
        let iter_obj = PyObject::new(
          PyObjectKind::Iterator {
            position: 0,
            iterated_obj: iterated_obj,
          },
          self.ctx.type_type.clone(),
        );
        self.push_value(iter_obj);
        None
      }
      bytecode::Instruction::ForIter => {
        // The top of stack contains the iterator, lets push it forward:
        let next_obj: Option<PyObjectRef> = {
          let top_of_stack = self.last_value();
          let ref_mut: RefMut<PyObject> = top_of_stack.deref().borrow_mut();
          // We require a mutable pyobject here to update the iterator:
          let mut iterator = ref_mut; // &mut PyObject = ref_mut.;
                                      // let () = iterator;
          iterator.nxt()
        };

        // Check the next object:
        match next_obj {
          Some(value) => {
            self.push_value(value);
          }
          None => {
            // Pop iterator from stack:
            self.pop_value();

            // End of for loop
            let end_label =
              if let Block::Loop { start: _, end } = self.last_block() {
                *end
              } else {
                panic!("Wrong block type")
              };
            self.jump(&end_label);
          }
        };
        None
      }
      bytecode::Instruction::MakeFunction => {
        let _qualified_name = self.pop_value();
        let code_obj = self.pop_value();
        // pop argc arguments
        // argument: name, args, globals
        let scope = self.current_frame().locals.clone();
        let obj = self.ctx.new_function(code_obj, scope);
        self.push_value(obj);
        None
      }
      bytecode::Instruction::CallFunction { count } => {
        let args: Vec<PyObjectRef> = self.pop_multiple(*count);
        // TODO: kwargs
        let args = PyFuncArgs { args: args };
        let func_ref = self.pop_value();

        // Call function:
        let func_result = self.invoke(func_ref, args);

        match func_result {
          Ok(value) => {
            self.push_value(value);
            None
          }
          Err(value) => {
            // Ripple exception upwards:
            Some(Err(value))
          }
        }
      }
      bytecode::Instruction::Jump { target } => {
        self.jump(target);
        None
      }
      bytecode::Instruction::JumpIf { target } => {
        let obj = self.pop_value();
        // TODO: determine if this value is True-ish:
        //if *v == NativeType::Boolean(true) {
        //    curr_frame.lasti = curr_frame.labels.get(target).unwrap().clone();
        //}
        let x = obj.borrow();
        let result: bool = match x.kind {
          PyObjectKind::Boolean { ref value } => *value,
          _ => {
            panic!("Not impl {:?}", x);
          }
        };
        if result {
          self.jump(target);
        }
        None
      }

      bytecode::Instruction::Raise { argc } => {
        let exception = match argc {
          1 => self.pop_value(),
          0 | 2 | 3 => panic!("Not implemented!"),
          _ => {
            panic!("Invalid paramter for RAISE_VARARGS, must be between 0 to 3")
          }
        };
        info!("Exception raised: {:?}", exception);
        Some(Err(exception))
      }

      bytecode::Instruction::Break => {
        let block = self.unwind_loop();
        if let Block::Loop { start: _, end } = block {
          self.jump(&end);
        }
        None
      }
      bytecode::Instruction::Pass => {
        // Ah, this is nice, just relax!
        None
      }
      bytecode::Instruction::Continue => {
        let block = self.unwind_loop();
        if let Block::Loop { start, end: _ } = block {
          self.jump(&start);
        } else {
          assert!(false);
        }
        None
      }
      bytecode::Instruction::PrintExpr => {
        let expr = self.pop_value();
        match expr.borrow().kind {
          PyObjectKind::PyNone => (),
          _ => {
            builtins::builtin_print(
              self,
              PyFuncArgs {
                args: vec![expr.clone()],
              },
            )
            .unwrap();
          }
        }
        None
      }
      bytecode::Instruction::LoadBuildClass => {
        let rustfunc = PyObject::new(
          PyObjectKind::RustFunction {
            function: builtins::builtin_build_class_,
          },
          self.ctx.type_type.clone(),
        );
        self.push_value(rustfunc);
        None
      }
      bytecode::Instruction::StoreLocals => {
        let locals = self.pop_value();
        let ref mut frame = self.current_frame();
        match frame.locals.borrow_mut().kind {
          PyObjectKind::Scope { ref mut scope } => {
            scope.locals = locals;
          }
          _ => panic!("We really expect our scope to be a scope!"),
        }
        None
      }
      _ => panic!("NOT IMPL {:?}", instruction),
    }
  }

  fn jump(&mut self, label: &bytecode::Label) {
    let current_frame = self.current_frame();
    let target_pc = current_frame.code.label_map[label];
    trace!(
      "program counter from {:?} to {:?}",
      current_frame.lasti,
      target_pc
    );
    current_frame.lasti = target_pc;
  }
}
