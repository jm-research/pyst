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
