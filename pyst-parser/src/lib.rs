extern crate lalrpop_util;

use lalrpop_util::lalrpop_mod;

lalrpop_util::lalrpop_mod!(pub calc);

#[test]
fn calculator1() {
  assert!(calc::TermParser::new().parse("22").is_ok());
  assert!(calc::TermParser::new().parse("(22)").is_ok());
  assert!(calc::TermParser::new().parse("((((22))))").is_ok());
  assert!(calc::TermParser::new().parse("((22)").is_err());
}

#[test]
fn test_langbase() {
  let mut chars = "abc".char_indices();

  let chr0 = chars.next().map(|x| x.1);
  println!("{:?}", chr0.unwrap());
}

#[macro_use]
extern crate log;

pub mod lexer;

pub mod token;