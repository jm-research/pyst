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
    let source = String::from(r"avariable = 99 + 2 - 0");
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

  macro_rules! test_indentation_with_eol {
    ($($name:ident: $eol:expr,)*) => {
        $(
        #[test]
        fn $name() {
            let source = String::from(format!("def foo():{}   return 99{}{}", $eol, $eol, $eol));
            let tokens = lex_source(&source);
            assert_eq!(
                tokens,
                vec![
                    Tok::Def,
                    Tok::Name {
                        name: String::from("foo"),
                    },
                    Tok::Lpar,
                    Tok::Rpar,
                    Tok::Colon,
                    Tok::Newline,
                    Tok::Indent,
                    Tok::Return,
                    Tok::Number { value: "99".to_string() },
                    Tok::Newline,
                    Tok::Dedent,
                ]
            );
        }
        )*
    };
}

  test_indentation_with_eol! {
    test_indentation_windows_eol: WINDOWS_EOL,
    test_indentation_mac_eol: MAC_EOL,
    test_indentation_unix_eol: UNIX_EOL,
  }

  macro_rules! test_double_dedent_with_eol {
    ($($name:ident: $eol:expr,)*) => {
    $(
      #[test]
      fn $name() {
        let source = String::from(format!("def foo():{} if x:{}{}  return 99{}{}", $eol, $eol, $eol, $eol, $eol));
        let tokens = lex_source(&source);
        assert_eq!(
          tokens,
          vec![
            Tok::Def,
            Tok::Name {
              name: String::from("foo"),
            },
            Tok::Lpar,
            Tok::Rpar,
            Tok::Colon,
            Tok::Newline,
            Tok::Indent,
            Tok::If,
            Tok::Name {
              name: String::from("x"),
            },
            Tok::Colon,
            Tok::Newline,
            Tok::Indent,
            Tok::Return,
            Tok::Number { value: "99".to_string() },
            Tok::Newline,
            Tok::Dedent,
            Tok::Dedent,
          ]
        );
      }
    )*
    }
}

  test_double_dedent_with_eol! {
    test_double_dedent_windows_eol: WINDOWS_EOL,
    test_double_dedent_mac_eol: MAC_EOL,
    test_double_dedent_unix_eol: UNIX_EOL,
  }

  macro_rules! test_newline_in_brackets {
    ($($name:ident: $eol:expr,)*) => {
    $(
      #[test]
      fn $name() {
        let source = String::from(format!("x = [{}    1,2{}]{}", $eol, $eol, $eol));
        let tokens = lex_source(&source);
        assert_eq!(
          tokens,
          vec![
            Tok::Name {
              name: String::from("x"),
            },
            Tok::Equal,
            Tok::Lsqb,
            Tok::Number { value: "1".to_string() },
            Tok::Comma,
            Tok::Number { value: "2".to_string() },
            Tok::Rsqb,
            Tok::Newline,
          ]
        );
      }
    )*
    };
}

  test_newline_in_brackets! {
    test_newline_in_brackets_windows_eol: WINDOWS_EOL,
    test_newline_in_brackets_mac_eol: MAC_EOL,
    test_newline_in_brackets_unix_eol: UNIX_EOL,
  }

  #[test]
  fn test_operators() {
    let source = String::from("//////=/ /");
    let tokens = lex_source(&source);
    assert_eq!(
      tokens,
      vec![
        Tok::DoubleSlash,
        Tok::DoubleSlash,
        Tok::DoubleSlashEqual,
        Tok::Slash,
        Tok::Slash,
      ]
    );
  }

  #[test]
  fn test_string() {
    let source = String::from(r#""double" 'single' 'can\'t' "\\\"" '\t\r\n' '\g'"#);
    let tokens = lex_source(&source);
    assert_eq!(
      tokens,
      vec![
        Tok::String {
          value: String::from("double"),
        },
        Tok::String {
          value: String::from("single"),
        },
        Tok::String {
          value: String::from("can't"),
        },
        Tok::String {
          value: String::from("\\\""),
        },
        Tok::String {
          value: String::from("\t\r\n"),
        },
        Tok::String {
          value: String::from("\\g"),
        },
      ]
    );
  }

  macro_rules! test_string_continuation {
    ($($name:ident: $eol:expr,)*) => {
    $(
      #[test]
      fn $name() {
        let source = String::from(format!("\"abc\\{}def\"", $eol));
        let tokens = lex_source(&source);
        assert_eq!(
          tokens,
          vec![
            Tok::String {
              value: String::from("abcdef"),
            },
          ]
        )
      }
    )*
    }
}

  test_string_continuation! {
    test_string_continuation_windows_eol: WINDOWS_EOL,
    test_string_continuation_mac_eol: MAC_EOL,
    test_string_continuation_unix_eol: UNIX_EOL,
  }
}
