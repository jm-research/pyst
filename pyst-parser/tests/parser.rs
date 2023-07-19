extern crate pyst_parser;

#[cfg(test)]
mod tests {
  use pyst_parser::ast;
  use pyst_parser::parser::parse_program;
  use pyst_parser::parser::parse_statement;

  #[test]
  fn test_parse_empty() {
    let parse_ast = parse_program(&String::from("\n"));

    assert_eq!(parse_ast, Ok(ast::Program { statements: vec![] }))
  }

  #[test]
  fn test_parse_print_hello() {
    let source = String::from("print('Hello world')\n");
    let parse_ast = parse_program(&source).unwrap();
    assert_eq!(
      parse_ast,
      ast::Program {
        statements: vec![ast::LocatedStatement {
          location: ast::Location::new(1, 1),
          node: ast::Statement::Expression {
            expression: ast::Expression::Call {
              function: Box::new(ast::Expression::Identifier {
                name: String::from("print"),
              }),
              args: vec![ast::Expression::String {
                value: String::from("Hello world"),
              },],
            },
          },
        },],
      }
    );
  }

  #[test]
  fn test_parse_print_2() {
    let source = String::from("print('Hello world', 2)\n");
    let parse_ast = parse_program(&source).unwrap();
    assert_eq!(
      parse_ast,
      ast::Program {
        statements: vec![ast::LocatedStatement {
          location: ast::Location::new(1, 1),
          node: ast::Statement::Expression {
            expression: ast::Expression::Call {
              function: Box::new(ast::Expression::Identifier {
                name: String::from("print"),
              }),
              args: vec![
                ast::Expression::String {
                  value: String::from("Hello world"),
                },
                ast::Expression::Number {
                  value: ast::Number::Integer { value: 2 },
                },
              ],
            },
          },
        },],
      }
    );
  }

  #[test]
  fn test_parse_if_elif_else() {
    let source = String::from("if 1: 10\nelif 2: 20\nelse: 30\n");
    let parse_ast = parse_statement(&source).unwrap();
    assert_eq!(
      parse_ast,
      ast::LocatedStatement {
        location: ast::Location::new(1, 1),
        node: ast::Statement::If {
          test: ast::Expression::Number {
            value: ast::Number::Integer { value: 1 },
          },
          body: vec![ast::LocatedStatement {
            location: ast::Location::new(1, 7),
            node: ast::Statement::Expression {
              expression: ast::Expression::Number {
                value: ast::Number::Integer { value: 10 },
              }
            },
          },],
          orelse: Some(vec![ast::LocatedStatement {
            location: ast::Location::new(2, 1),
            node: ast::Statement::If {
              test: ast::Expression::Number {
                value: ast::Number::Integer { value: 2 },
              },
              body: vec![ast::LocatedStatement {
                location: ast::Location::new(2, 9),
                node: ast::Statement::Expression {
                  expression: ast::Expression::Number {
                    value: ast::Number::Integer { value: 20 },
                  },
                },
              },],
              orelse: Some(vec![ast::LocatedStatement {
                location: ast::Location::new(3, 7),
                node: ast::Statement::Expression {
                  expression: ast::Expression::Number {
                    value: ast::Number::Integer { value: 30 },
                  },
                },
              },]),
            }
          },]),
        }
      }
    );
  }

  #[test]
  fn test_parse_lambda() {
    let source = String::from("lambda x, y: x * y\n"); // lambda(x, y): x * y");
    let parse_ast = parse_statement(&source);
    assert_eq!(
      parse_ast,
      Ok(ast::LocatedStatement {
        location: ast::Location::new(1, 1),
        node: ast::Statement::Expression {
          expression: ast::Expression::Lambda {
            args: vec![String::from("x"), String::from("y")],
            body: Box::new(ast::Expression::Binop {
              a: Box::new(ast::Expression::Identifier {
                name: String::from("x"),
              }),
              op: ast::Operator::Mult,
              b: Box::new(ast::Expression::Identifier {
                name: String::from("y"),
              })
            })
          }
        }
      })
    )
  }

  #[test]
  fn test_parse_class() {
    let source = String::from("class Foo(A, B):\n def __init__(self):\n  pass\n");
    assert_eq!(
      parse_statement(&source),
      Ok(ast::LocatedStatement {
        location: ast::Location::new(1, 1),
        node: ast::Statement::ClassDef {
          name: String::from("Foo"),
          args: vec![String::from("A"), String::from("B")],
          body: vec![ast::LocatedStatement {
            location: ast::Location::new(2, 2),
            node: ast::Statement::FunctionDef {
              name: String::from("__init__"),
              args: vec![String::from("self")],
              body: vec![ast::LocatedStatement {
                location: ast::Location::new(3, 3),
                node: ast::Statement::Pass,
              }],
            }
          }],
        }
      })
    )
  }
}
