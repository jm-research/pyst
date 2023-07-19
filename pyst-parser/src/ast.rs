pub use super::lexer::Location;

#[derive(Debug, PartialEq)]
pub struct Program {
  pub statements: Vec<LocatedStatement>,
}

#[derive(Debug, PartialEq)]
pub struct SingleImport {
  pub module: String,
  pub symbol: Option<String>,
  pub alias: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Located<T> {
  pub location: Location,
  pub node: T,
}

pub type LocatedStatement = Located<Statement>;

#[derive(Debug, PartialEq)]
pub enum Statement {
  Break,
  Continue,
  Return {
    value: Option<Vec<Expression>>,
  },
  Import {
    import_parts: Vec<SingleImport>,
  },
  Pass,
  Assert {
    test: Expression,
    msg: Option<Expression>,
  },
  Delete {
    targets: Vec<Expression>,
  },
  Assign {
    targets: Vec<Expression>,
    value: Expression,
  },
  AugAssign {
    target: Expression,
    op: Operator,
    value: Expression,
  },
  Expression {
    expression: Expression,
  },
  If {
    test: Expression,
    body: Vec<LocatedStatement>,
    orelse: Option<Vec<LocatedStatement>>,
  },
  While {
    test: Expression,
    body: Vec<LocatedStatement>,
    orelse: Option<Vec<LocatedStatement>>,
  },
  With {
    items: Expression,
    body: Vec<LocatedStatement>,
  },
  For {
    target: Vec<Expression>,
    iter: Vec<Expression>,
    body: Vec<LocatedStatement>,
    orelse: Option<Vec<LocatedStatement>>,
  },
  Raise {
    expression: Option<Expression>,
  },
  Try {
    body: Vec<LocatedStatement>,
    handlers: Vec<ExceptHandler>,
    orelse: Option<Vec<LocatedStatement>>,
    finalbody: Option<Vec<LocatedStatement>>,
  },
  ClassDef {
    name: String,
    body: Vec<LocatedStatement>,
    args: Vec<String>,
    // TODO: docstring: String,
  },
  FunctionDef {
    name: String,
    args: Vec<String>,
    // docstring: String,
    body: Vec<LocatedStatement>,
  },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
  BoolOp {
    a: Box<Expression>,
    op: BooleanOperator,
    b: Box<Expression>,
  },
  Binop {
    a: Box<Expression>,
    op: Operator,
    b: Box<Expression>,
  },
  Subscript {
    a: Box<Expression>,
    b: Box<Expression>,
  },
  Unop {
    op: UnaryOperator,
    a: Box<Expression>,
  },
  Compare {
    a: Box<Expression>,
    op: Comparison,
    b: Box<Expression>,
  },
  Attribute {
    value: Box<Expression>,
    name: String,
  },
  Call {
    function: Box<Expression>,
    args: Vec<Expression>,
  },
  Number {
    value: Number,
  },
  List {
    elements: Vec<Expression>,
  },
  Tuple {
    elements: Vec<Expression>,
  },
  Dict {
    elements: Vec<(Expression, Expression)>,
  },
  Slice {
    elements: Vec<Expression>,
  },
  String {
    value: String,
  },
  Identifier {
    name: String,
  },
  Lambda {
    args: Vec<String>,
    body: Box<Expression>,
  },
  True,
  False,
  PyNone,
}

#[derive(Debug, PartialEq)]
pub struct ExceptHandler {
  pub typ: Option<Expression>,
  pub name: Option<String>,
  pub body: Vec<LocatedStatement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
  Add,
  Sub,
  Mult,
  MatMult,
  Div,
  Mod,
  Pow,
  LShift,
  RShift,
  BitOr,
  BitXor,
  BitAnd,
  FloorDiv,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BooleanOperator {
  And,
  Or,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
  Neg,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Comparison {
  Equal,
  NotEqual,
  Less,
  LessOrEqual,
  Greater,
  GreaterOrEqual,
  In,
  NotIn,
  Is,
  IsNot,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Number {
  Integer { value: i32 },
  Float { value: f64 },
}
