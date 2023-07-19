use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::ast;
use super::lexer;
use super::python;

pub fn read_file(filename: &Path) -> Result<String, String> {
  match File::open(&filename) {
    Ok(mut file) => {
      let mut s = String::new();

      match file.read_to_string(&mut s) {
        Err(why) => Err(format!("Reading file failed: {}", why)),
        Ok(_) => Ok(s),
      }
    }
    Err(why) => Err(format!("Opening file failed: {}", why)),
  }
}

pub fn parse(filename: &Path) -> Result<ast::Program, String> {
  info!("Parsing: {}", filename.display());
  match read_file(filename) {
    Ok(txt) => {
      debug!("Read contents of file: {}", txt);
      parse_program(&txt)
    }
    Err(msg) => Err(msg),
  }
}

pub fn parse_program(source: &String) -> Result<ast::Program, String> {
  let lxr = lexer::Lexer::new(&source);
  match python::ProgramParser::new().parse(lxr) {
    Err(lalrpop_util::ParseError::UnrecognizedToken {
      token: _,
      expected: _,
    }) => Err(String::from("Unexpected end of input.")),
    Err(why) => Err(String::from(format!("{:?}", why))),
    Ok(p) => Ok(p),
  }
}

pub fn parse_statement(source: &String) -> Result<ast::LocatedStatement, String> {
  let lxr = lexer::Lexer::new(&source);
  match python::StatementParser::new().parse(lxr) {
    Err(why) => Err(String::from(format!("{:?}", why))),
    Ok(p) => Ok(p),
  }
}

pub fn parse_expression(source: &String) -> Result<ast::Expression, String> {
  let lxr = lexer::Lexer::new(&source);
  match python::ExpressionParser::new().parse(lxr) {
    Err(why) => Err(String::from(format!("{:?}", why))),
    Ok(p) => Ok(p),
  }
}
