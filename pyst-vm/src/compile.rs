extern crate pyst_parser;

use self::pyst_parser::{ast, parser};
use super::bytecode::{self, CodeObject, Instruction};
use super::pyobject::{PyObject, PyObjectKind, PyObjectRef};
use super::vm::VirtualMachine;

struct Compiler {
  code_object_stack: Vec<CodeObject>,
  nxt_label: usize,
  current_source_location: ast::Location,
}

pub fn compile(
  vm: &mut VirtualMachine,
  source: &String,
  mode: Mode,
) -> Result<PyObjectRef, String> {
  let mut compiler = Compiler::new();
  compiler.push_new_code_object();
  match mode {
    Mode::Exec => match parser::parse_program(source) {
      Ok(ast) => {
        compiler.compile_program(&ast);
      }
      Err(msg) => return Err(msg),
    },
    Mode::Eval => match parser::parse_statement(source) {
      Ok(statement) => {
        if let &ast::Statement::Expression { ref expression } = &statement.node
        {
          compiler.compile_expression(expression);
          compiler.emit(Instruction::ReturnValue);
        } else {
          return Err("Expecting expression, got statement".to_string());
        }
      }
      Err(msg) => return Err(msg),
    },
    Mode::Single => match parser::parse_program(source) {
      Ok(ast) => {
        for statement in ast.statements {
          if let &ast::Statement::Expression { ref expression } =
            &statement.node
          {
            compiler.compile_expression(expression);
            compiler.emit(Instruction::PrintExpr);
          } else {
            compiler.compile_statement(&statement);
          }
        }
        compiler.emit(Instruction::LoadConst {
          value: bytecode::Constant::None,
        });
        compiler.emit(Instruction::ReturnValue);
      }
      Err(msg) => return Err(msg),
    },
  };

  let code = compiler.pop_code_object();
  trace!("Compilation completed: {:?}", code);
  Ok(PyObject::new(
    PyObjectKind::Code { code: code },
    vm.get_type(),
  ))
}

pub enum Mode {
  Exec,
  Eval,
  Single,
}

type Label = usize;

impl Compiler {
  fn new() -> Self {
    Compiler {
      code_object_stack: Vec::new(),
      nxt_label: 0,
      current_source_location: ast::Location::default(),
    }
  }

  fn push_new_code_object(&mut self) {
    self.code_object_stack.push(CodeObject::new(Vec::new()));
  }

  fn pop_code_object(&mut self) -> CodeObject {
    self.code_object_stack.pop().unwrap()
  }

  fn compile_program(&mut self, program: &ast::Program) {
    let size_before = self.code_object_stack.len();
    self.compile_statements(&program.statements);
    assert!(self.code_object_stack.len() == size_before);

    // Emit None at end:
    self.emit(Instruction::LoadConst {
      value: bytecode::Constant::None,
    });
    self.emit(Instruction::ReturnValue);
  }

  fn compile_statements(&mut self, statements: &Vec<ast::LocatedStatement>) {
    for statement in statements {
      self.compile_statement(statement)
    }
  }

  fn compile_statement(&mut self, statement: &ast::LocatedStatement) {
    trace!("Compiling {:?}", statement);
    self.set_source_location(&statement.location);

    match &statement.node {
      ast::Statement::Import { import_parts } => {
        for ast::SingleImport {
          module,
          symbol,
          alias,
        } in import_parts
        {
          self.emit(Instruction::Import {
            name: module.clone(),
            symbol: symbol.clone().map(|s| s.clone()),
          });
          self.emit(Instruction::StoreName {
            name: match alias {
              Some(alias) => alias.clone(),
              None => match symbol {
                Some(symbol) => symbol.clone(),
                None => module.clone(),
              },
            },
          });
        }
      }
      ast::Statement::Expression { expression } => {
        self.compile_expression(expression);

        // Pop result of stack, since we not use it:
        self.emit(Instruction::Pop);
      }
      ast::Statement::If { test, body, orelse } => {
        let end_label = self.new_label();
        match orelse {
          None => {
            // Only if:
            self.compile_test(test, end_label);
            self.compile_statements(body);
          }
          Some(statements) => {
            // if - else:
            let else_label = self.new_label();
            self.compile_test(test, else_label);
            self.compile_statements(body);
            self.emit(Instruction::Jump { target: end_label });

            // else:
            self.set_label(else_label);
            self.compile_statements(statements);
          }
        }
        self.set_label(end_label);
      }
      ast::Statement::While {
        test,
        body,
        orelse: _,
      } => {
        // TODO: Handle while-loop else clauses
        let start_label = self.new_label();
        let end_label = self.new_label();
        self.emit(Instruction::SetupLoop {
          start: start_label,
          end: end_label,
        });

        self.set_label(start_label);

        self.compile_test(test, end_label);
        self.compile_statements(body);
        self.emit(Instruction::Jump {
          target: start_label,
        });
        self.set_label(end_label);
      }
      ast::Statement::With { items: _, body: _ } => {
        // TODO
      }
      ast::Statement::For {
        target,
        iter,
        body,
        orelse: _,
      } => {
        // TODO: Handle for loop else clauses
        // The thing iterated:
        for i in iter {
          self.compile_expression(i);
        }

        // Retrieve iterator
        self.emit(Instruction::GetIter);

        // Start loop
        let start_label = self.new_label();
        let end_label = self.new_label();
        self.emit(Instruction::SetupLoop {
          start: start_label,
          end: end_label,
        });
        self.set_label(start_label);
        self.emit(Instruction::ForIter);

        // Start of loop iteration, set targets:
        for t in target {
          match t {
            ast::Expression::Identifier { name } => {
              self.emit(Instruction::StoreName {
                name: name.to_string(),
              });
            }
            _ => panic!("Not impl"),
          }
        }

        // Body of loop:
        self.compile_statements(body);
        self.emit(Instruction::Jump {
          target: start_label,
        });
        self.set_label(end_label);
        self.emit(Instruction::PopBlock);
      }
      ast::Statement::Raise { expression } => match expression {
        Some(value) => {
          self.compile_expression(value);
          self.emit(Instruction::Raise { argc: 1 });
        }
        None => {
          unimplemented!();
        }
      },
      ast::Statement::Try {
        body,
        handlers,
        orelse,
        finalbody,
      } => {
        let mut handler_label = self.new_label();
        let finally_label = self.new_label();
        let else_label = self.new_label();
        // try:
        self.emit(Instruction::SetupExcept {
          handler: handler_label,
        });
        self.compile_statements(body);
        self.emit(Instruction::PopBlock);
        self.emit(Instruction::Jump { target: else_label });

        // except handlers:
        self.set_label(handler_label);
        handler_label = self.new_label();
        for handler in handlers {
          // Check if this handler can handle the exception:

          // TODO: self.emit(isinstance()) start of hack
          self.emit(Instruction::LoadConst {
            value: bytecode::Constant::None,
          });
          // End of hack
          self.emit(Instruction::JumpIf {
            target: handler_label,
          });

          // We have a match
          self.compile_statements(&handler.body);
          self.emit(Instruction::Jump {
            target: finally_label,
          });

          // Emit a new label for the next handler
          self.set_label(handler_label);
          handler_label = self.new_label();
        }
        self.emit(Instruction::Jump {
          target: handler_label,
        });

        // We successfully ran the try block:
        // else:
        self.set_label(else_label);
        if let Some(statements) = orelse {
          self.compile_statements(statements);
        }

        // finally:
        self.set_label(finally_label);
        if let Some(statements) = finalbody {
          self.compile_statements(statements);
        }

        // unimplemented!();
      }
      ast::Statement::FunctionDef { name, args, body } => {
        // Create bytecode for this function:
        self.code_object_stack.push(CodeObject::new(args.to_vec()));
        self.compile_statements(body);

        // Emit None at end:
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::None,
        });
        self.emit(Instruction::ReturnValue);

        let code = self.code_object_stack.pop().unwrap();
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Code { code: code },
        });
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::String {
            value: name.clone(),
          },
        });

        // Turn code object into function object:
        self.emit(Instruction::MakeFunction);
        self.emit(Instruction::StoreName {
          name: name.to_string(),
        });
      }
      ast::Statement::ClassDef { name, body, args } => {
        self.emit(Instruction::LoadBuildClass);
        self
          .code_object_stack
          .push(CodeObject::new(vec![String::from("__locals__")]));
        self.emit(Instruction::LoadName {
          name: String::from("__locals__"),
        });
        self.emit(Instruction::StoreLocals);
        self.compile_statements(body);
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::None,
        });
        self.emit(Instruction::ReturnValue);

        let code = self.code_object_stack.pop().unwrap();
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Code { code: code },
        });
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::String {
            value: name.clone(),
          },
        });
        // Turn code object into function object:
        self.emit(Instruction::MakeFunction);

        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::String {
            value: name.clone(),
          },
        });

        for base in args {
          self.emit(Instruction::LoadName { name: base.clone() });
        }
        self.emit(Instruction::CallFunction {
          count: 2 + args.len(),
        });

        self.emit(Instruction::StoreName {
          name: name.to_string(),
        });
      }
      ast::Statement::Assert { test, msg } => {
        // TODO: if some flag, ignore all assert statements!

        self.compile_expression(test);

        // if true, jump over raise:
        let end_label = self.new_label();
        self.emit(Instruction::JumpIf { target: end_label });

        self.emit(Instruction::LoadName {
          name: String::from("AssertionError"),
        });
        match msg {
          Some(e) => {
            self.compile_expression(e);
            self.emit(Instruction::CallFunction { count: 1 });
          }
          None => {
            self.emit(Instruction::CallFunction { count: 0 });
          }
        }
        self.set_label(end_label);
      }
      ast::Statement::Break => {
        self.emit(Instruction::Break);
      }
      ast::Statement::Continue => {
        self.emit(Instruction::Continue);
      }
      ast::Statement::Return { value } => {
        match value {
          Some(e) => {
            let size = e.len();
            for v in e {
              self.compile_expression(v);
            }

            // If we have more than 1 return value, make it a tuple:
            if size > 1 {
              self.emit(Instruction::BuildTuple { size });
            }
          }
          None => {
            // TODO: Put none on stack
          }
        }

        self.emit(Instruction::ReturnValue);
      }
      ast::Statement::Assign { targets, value } => {
        self.compile_expression(value);

        for target in targets {
          self.compile_store(target);
        }
      }
      ast::Statement::AugAssign { target, op, value } => {
        self.compile_expression(target);
        self.compile_expression(value);

        // Perform operation:
        self.compile_op(op);
        self.compile_store(target);
      }
      ast::Statement::Delete { targets: _ } => {
        // TODO: Remove the given names from the scope
        // self.emit(Instruction::DeleteName);
      }
      ast::Statement::Pass => {
        self.emit(Instruction::Pass);
      }
    }
  }

  fn compile_store(&mut self, target: &ast::Expression) {
    match target {
      ast::Expression::Identifier { name } => {
        self.emit(Instruction::StoreName {
          name: name.to_string(),
        });
      }
      ast::Expression::Subscript { a, b } => {
        self.compile_expression(a);
        self.compile_expression(b);
        self.emit(Instruction::StoreSubscript);
      }
      ast::Expression::Attribute { value, name } => {
        self.compile_expression(value);
        self.emit(Instruction::StoreAttr {
          name: name.to_string(),
        });
      }
      _ => {
        panic!("WTF: {:?}", target);
      }
    }
  }

  fn compile_op(&mut self, op: &ast::Operator) {
    let i = match op {
      ast::Operator::Add => bytecode::BinaryOperator::Add,
      ast::Operator::Sub => bytecode::BinaryOperator::Subtract,
      ast::Operator::Mult => bytecode::BinaryOperator::Multiply,
      ast::Operator::MatMult => bytecode::BinaryOperator::MatrixMultiply,
      ast::Operator::Div => bytecode::BinaryOperator::Divide,
      ast::Operator::FloorDiv => bytecode::BinaryOperator::FloorDivide,
      ast::Operator::Mod => bytecode::BinaryOperator::Modulo,
      ast::Operator::Pow => bytecode::BinaryOperator::Power,
      ast::Operator::LShift => bytecode::BinaryOperator::Lshift,
      ast::Operator::RShift => bytecode::BinaryOperator::Rshift,
      ast::Operator::BitOr => bytecode::BinaryOperator::Or,
      ast::Operator::BitXor => bytecode::BinaryOperator::Xor,
      ast::Operator::BitAnd => bytecode::BinaryOperator::And,
    };
    self.emit(Instruction::BinaryOperation { op: i });
  }

  fn compile_test(&mut self, expression: &ast::Expression, not_label: Label) {
    // Compile expression for test, and jump to label if false
    match expression {
      ast::Expression::BoolOp { a, op, b } => match op {
        ast::BooleanOperator::And => {
          self.compile_test(a, not_label);
          self.compile_test(b, not_label);
        }
        ast::BooleanOperator::Or => {
          // TODO: Implement boolean or
          // TODO: implement short circuit code by fiddeling with the labels
          self.new_label();
          self.compile_test(a, not_label);
          self.compile_test(b, not_label);
          panic!("Not impl");
        }
      },
      _ => {
        // If all else fail, fall back to simple checking of boolean value:
        self.compile_expression(expression);
        self.emit(Instruction::UnaryOperation {
          op: bytecode::UnaryOperator::Not,
        });
        self.emit(Instruction::JumpIf { target: not_label });
      }
    }
  }

  fn compile_expression(&mut self, expression: &ast::Expression) {
    trace!("Compiling {:?}", expression);
    match expression {
      ast::Expression::Call { function, args } => {
        self.compile_expression(&*function);
        let count = args.len();
        for arg in args {
          self.compile_expression(arg)
        }
        self.emit(Instruction::CallFunction { count: count });
      }
      ast::Expression::BoolOp { a: _, op: _, b: _ } => {
        let not_label = self.new_label();
        let end_label = self.new_label();
        self.compile_test(expression, not_label);
        // Load const True
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Boolean { value: true },
        });
        self.emit(Instruction::Jump { target: end_label });

        self.set_label(not_label);
        // Load const False
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Boolean { value: false },
        });
        self.set_label(end_label);
      }
      ast::Expression::Binop { a, op, b } => {
        self.compile_expression(&*a);
        self.compile_expression(&*b);

        // Perform operation:
        self.compile_op(op);
      }
      ast::Expression::Subscript { a, b } => {
        self.compile_expression(&*a);
        self.compile_expression(&*b);
        self.emit(Instruction::BinaryOperation {
          op: bytecode::BinaryOperator::Subscript,
        });
      }
      ast::Expression::Unop { op, a } => {
        self.compile_expression(&*a);

        // Perform operation:
        let i = match op {
          ast::UnaryOperator::Neg => bytecode::UnaryOperator::Minus,
        };
        let i = Instruction::UnaryOperation { op: i };
        self.emit(i);
      }
      ast::Expression::Attribute { value, name } => {
        self.compile_expression(&*value);
        self.emit(Instruction::LoadAttr {
          name: name.to_string(),
        });
      }
      ast::Expression::Compare { a, op, b } => {
        self.compile_expression(&*a);
        self.compile_expression(&*b);

        let i = match op {
          ast::Comparison::Equal => bytecode::ComparisonOperator::Equal,
          ast::Comparison::NotEqual => bytecode::ComparisonOperator::NotEqual,
          ast::Comparison::Less => bytecode::ComparisonOperator::Less,
          ast::Comparison::LessOrEqual => {
            bytecode::ComparisonOperator::LessOrEqual
          }
          ast::Comparison::Greater => bytecode::ComparisonOperator::Greater,
          ast::Comparison::GreaterOrEqual => {
            bytecode::ComparisonOperator::GreaterOrEqual
          }
          ast::Comparison::In => bytecode::ComparisonOperator::In,
          ast::Comparison::NotIn => bytecode::ComparisonOperator::NotIn,
          ast::Comparison::Is => bytecode::ComparisonOperator::Is,
          ast::Comparison::IsNot => bytecode::ComparisonOperator::IsNot,
        };
        let i = Instruction::CompareOperation { op: i };
        self.emit(i);
      }
      ast::Expression::Number { value } => {
        let const_value = match value {
          ast::Number::Integer { value } => {
            bytecode::Constant::Integer { value: *value }
          }
          ast::Number::Float { value } => {
            bytecode::Constant::Float { value: *value }
          }
        };
        self.emit(Instruction::LoadConst { value: const_value });
      }
      ast::Expression::List { elements } => {
        let size = elements.len();
        for element in elements {
          self.compile_expression(element);
        }
        self.emit(Instruction::BuildList { size: size });
      }
      ast::Expression::Tuple { elements } => {
        let size = elements.len();
        for element in elements {
          self.compile_expression(element);
        }
        self.emit(Instruction::BuildTuple { size: size });
      }
      ast::Expression::Dict { elements } => {
        let size = elements.len();
        for (key, value) in elements {
          self.compile_expression(key);
          self.compile_expression(value);
        }
        self.emit(Instruction::BuildMap { size: size });
      }
      ast::Expression::Slice { elements } => {
        let size = elements.len();
        for element in elements {
          self.compile_expression(element);
        }
        self.emit(Instruction::BuildSlice { size: size });
      }
      ast::Expression::True => {
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Boolean { value: true },
        });
      }
      ast::Expression::False => {
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Boolean { value: false },
        });
      }
      ast::Expression::PyNone => {
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::None,
        });
      }
      ast::Expression::String { value } => {
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::String {
            value: value.to_string(),
          },
        });
      }
      ast::Expression::Identifier { name } => {
        self.emit(Instruction::LoadName {
          name: name.to_string(),
        });
      }
      ast::Expression::Lambda { args, body } => {
        self.code_object_stack.push(CodeObject::new(args.to_vec()));
        self.compile_expression(body);
        self.emit(Instruction::ReturnValue);
        let code = self.code_object_stack.pop().unwrap();
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::Code { code: code },
        });
        self.emit(Instruction::LoadConst {
          value: bytecode::Constant::String {
            value: String::from("<lambda>"),
          },
        });
        // Turn code object into function object:
        self.emit(Instruction::MakeFunction);
      }
    }
  }

  // Low level helper functions:
  fn emit(&mut self, instruction: Instruction) {
    self.current_code_object().instructions.push(instruction);
    // TODO: insert source filename
    let location = self.current_source_location.clone();
    self.current_code_object().locations.push(location);
  }

  fn current_code_object(&mut self) -> &mut CodeObject {
    self.code_object_stack.last_mut().unwrap()
  }

  // Generate a new label
  fn new_label(&mut self) -> Label {
    let l = self.nxt_label;
    self.nxt_label += 1;
    l
  }

  // Assign current position the given label
  fn set_label(&mut self, label: Label) {
    let position = self.current_code_object().instructions.len();
    // assert!(label not in self.label_map)
    self.current_code_object().label_map.insert(label, position);
  }

  fn set_source_location(&mut self, location: &ast::Location) {
    self.current_source_location = location.clone();
  }
}
