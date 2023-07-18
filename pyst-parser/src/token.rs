// based on token.h from CPython source:
#[derive(Debug, PartialEq)]
pub enum Tok {
  Name { name: String },
  Number { value: String },
  String { value: String },
  Newline,
  Indent,
  Dedent,
  Lpar,             // '('
  Rpar,             // ')'
  Lsqb,             // '['
  Rsqb,             // ']'
  Colon,            // ':'
  Comma,            // ','
  Semi,             // ';'
  Plus,             // '+'
  Minus,            // '-'
  Star,             // '*'
  Slash,            // '/'
  Vbar,             // '|'
  Amper,            // '&'
  Less,             // '<'
  Greater,          // '>'
  Equal,            // '='
  Dot,              // '.'
  Percent,          // '%'
  Lbrace,           // '{'
  Rbrace,           // '}'
  EqEqual,          // '=='
  NotEqual,         // '!='
  LessEqual,        // '<='
  GreaterEqual,     // '>='
  Tilde,            // '~'
  CircumFlex,       // '^'
  LeftShift,        // '<<'
  RightShift,       // '>>'
  DoubleStar,       // '**'
  PlusEqual,        // '+='
  MinusEqual,       // '-='
  StarEqual,        // '*='
  SlashEqual,       // '/='
  PercentEqual,     // '%='
  AmperEqual,       // '&='
  VbarEqual,        // '|='
  CircumflexEqual,  // '^='
  LeftShiftEqual,   // '<<='
  RightShiftEqual,  // '>>='
  DoubleStarEqual,  // '**='
  DoubleSlash,      // '//'
  DoubleSlashEqual, // '//='
  At,               // '@'
  AtEqual,          // '@='
  Rarrow,           // '->'
  Ellipsis,         // '...'

  // Keywords (alphabetically):
  False,
  None,
  True,

  And,
  As,
  Assert,
  Break,
  Class,
  Continue,
  Def,
  Del,
  Elif,
  Else,
  Except,
  Finally,
  For,
  From,
  Global,
  If,
  Import,
  In,
  Is,
  Lambda,
  Nonlocal,
  Not,
  Or,
  Pass,
  Raise,
  Return,
  Try,
  While,
  With,
  Yield,
}
