extern crate pyst_parser;

#[cfg(test)]
mod tests {
  use pyst_parser::lexer::Lexer;
  use pyst_parser::token::Tok;
  use std::iter::FromIterator;

  const WINDOWS_EOL: &str = "\r\n";
  const MAC_EOL: &str = "\r";
  const UNIX_EOL: &str = "\n";

  pub fn lex_source(source: &String) -> Vec<Tok> {
    let lexer = Lexer::new(source);
    Vec::from_iter(lexer.map(|x| x.unwrap().1))
  }

  macro_rules! test_line_comment {
    ($($name:ident: $eol:expr,)*) => {
      $(
      #[test]
      fn $name() {
        let source = String::from(format!(r"99232  # {}", $eol));
        let tokens = lex_source(&source);
        assert_eq!(tokens, vec![Tok::Number { value: "99232".to_string() }]);
      }
      )*
    }
  }

  test_line_comment! {
    test_line_comment_long: " foo",
    test_line_comment_whitespace: "  ",
    test_line_comment_single_whitespace: " ",
    test_line_comment_empty: "",
  }

  macro_rules! test_comment_until_eol {
    ($($name:ident: $eol:expr,)*) => {
      $(
      #[test]
      fn $name() {
        let source = String::from(format!("123  # Foo{}456", $eol));
        let tokens = lex_source(&source);
        assert_eq!(
          tokens,
          vec![
              Tok::Number { value: "123".to_string() },
              Tok::Newline,
              Tok::Number { value: "456".to_string() },
          ]
        )
      }
      )*
    }
  }

  test_comment_until_eol! {
    test_comment_until_windows_eol: WINDOWS_EOL,
    test_comment_until_mac_eol: MAC_EOL,
    test_comment_until_unix_eol: UNIX_EOL,
  }

  #[test]
  fn test_assignment() {
    let source = String::from(r"avariable = 99 + 2-0");
    let tokens = lex_source(&source);
    assert_eq!(
      tokens,
      vec![
        Tok::Name {
          name: String::from("avariable"),
        },
        Tok::Equal,
        Tok::Number {
          value: "99".to_string()
        },
        Tok::Plus,
        Tok::Number {
          value: "2".to_string()
        },
        Tok::Minus,
        Tok::Number {
          value: "0".to_string()
        },
      ]
    );
  }
}
