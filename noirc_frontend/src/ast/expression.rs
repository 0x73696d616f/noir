use crate::{BlockStatement, Ident, Type};
use crate::token::{Keyword, Token, Attribute, SpannedToken};
use noirc_errors::{Spanned, Span};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExpressionKind {
    Ident(String), // an identifer can also produce a value. e.g. let x = y; y is an expression in this case
    Literal(Literal),
    Prefix(Box<PrefixExpression>),
    Infix(Box<InfixExpression>),
    Index(Box<IndexExpression>),
    Call(NoirPath, Box<CallExpression>), // Make Path Optional and so we only have one call expression
    Cast(Box<CastExpression>),
    Predicate(Box<InfixExpression>),
    For(Box<ForExpression>)
}

#[derive(Debug, Eq, Clone)]
pub struct Expression {
    pub kind : ExpressionKind,
    pub span : Span,
}

// This is important for tests. Two expressions are the same, iff their Kind is the same
// We are ignoring Span
impl PartialEq<Expression> for Expression {
    fn eq(&self, rhs: &Expression) -> bool {
        self.kind == rhs.kind
    }
}

impl Expression {
    pub fn into_ident(self) -> Option<Ident> {
        let identifier = match self.kind {
            ExpressionKind::Ident(x) => x,
            _=>return None
        };

        let ident = Ident(Spanned::from(self.span, identifier));
        return Some(ident)
    }
}

impl ExpressionKind {
    pub fn into_span(self, span : Span) -> Expression {
        Expression {
            span, 
            kind : self
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForExpression{
    pub identifier: Ident,
    pub start_range: Expression,
    pub end_range: Expression,
    pub block: BlockStatement,
}

impl ExpressionKind {
    pub fn infix(self) -> Option<InfixExpression> {
        match self {
            ExpressionKind::Infix(infix) => Some(*infix),
            ExpressionKind::Predicate(infix) => Some(*infix),
            _ => None,
        }
    }
    /// Converts an Expression to a u128
    /// The Expression must be a literal integer
    pub fn to_u128(&self) -> u128 {
        let integer = self.integer().expect("Expression is not an integer");
        integer as u128
    }

    fn integer(&self) -> Option<i128> {
        let literal = match self {
            ExpressionKind::Literal(literal) => literal,
            _ => return None,
        };

        match literal {
            Literal::Integer(integer) => Some(*integer),
            _=> None
        }
    }

    /// Returns true if the expression is a literal integer
    pub fn is_integer(&self) -> bool {
        self.integer().is_some()
    }
    
    pub fn identifier(&self) -> Option<String> {
        match self {
            ExpressionKind::Ident(x) => Some(x.clone()),
            _=> None
        }
    }
    /// Returns true if the expression is an identifier
    pub fn is_identifier(&self) -> bool {
        self.identifier().is_some()
    }

    /// Returns true if the expression can be used in a range expression
    /// Currently we only support Identifiers and constants literals
    pub fn can_be_used_range(&self) -> bool {
        self.is_identifier() || self.is_integer()
    }
}

pub type BinaryOp = Spanned<BinaryOpKind>;

#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Copy, Clone)]
pub enum BinaryOpKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Xor,
    As,
    // This is the only binary operator which cannot be used in a constrain statement
    Assign,
}

impl BinaryOpKind {
    /// Comparator operators return a 0 or 1
    /// When seen in the middle of an infix operator,
    /// they transform the infix expression into a predicate expression
    pub fn is_comparator(&self) -> bool {
        match self {
            BinaryOpKind::Equal |
            BinaryOpKind::NotEqual |
            BinaryOpKind::LessEqual |
            BinaryOpKind::Less |
            BinaryOpKind::Greater |
            BinaryOpKind::GreaterEqual => true, 
            _=> false
        }
    }
}

impl From<&Token> for BinaryOpKind {
    fn from(token: &Token) -> BinaryOpKind {
        match token {
            Token::Plus => BinaryOpKind::Add,
            Token::Ampersand => BinaryOpKind::And,
            Token::Caret => BinaryOpKind::Xor,
            Token::Pipe => BinaryOpKind::Or,
            Token::Minus => BinaryOpKind::Subtract,
            Token::Star => BinaryOpKind::Multiply,
            Token::Slash => BinaryOpKind::Divide,
            Token::Equal => BinaryOpKind::Equal,
            Token::NotEqual => BinaryOpKind::NotEqual,
            Token::Less => BinaryOpKind::Less,
            Token::LessEqual => BinaryOpKind::LessEqual,
            Token::Greater => BinaryOpKind::Greater,
            Token::GreaterEqual => BinaryOpKind::GreaterEqual,
            Token::Assign => BinaryOpKind::Assign,
            Token::Keyword(Keyword::As) => BinaryOpKind::As,
            _ => panic!(
                "The token:  \" {} \"does not seem to be a binary operation ",
                token
            ),
        }
    }
}

impl From<Token> for BinaryOpKind {
    fn from(token : Token) -> BinaryOpKind {
        BinaryOpKind::from(&token)
    }
}
impl From<&SpannedToken> for BinaryOp {
    fn from(st : &SpannedToken) -> BinaryOp {
        Spanned::from(st.into_span(), BinaryOpKind::from(st.token()))
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Copy, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
}

impl UnaryOp {
    /// Converts a token to a unary operator
    /// If you want the parser to recognise another Token as being a prefix operator, it is defined here
    pub fn from(token: &Token) -> UnaryOp {
        match token {
            Token::Minus => UnaryOp::Minus,
            Token::Bang => UnaryOp::Not,
            _ => panic!(
                "The token {} has not been linked to a unary operator",
                token
            ),
        }
    }

}
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Literal {
    Array(ArrayLiteral),
    Bool(bool),
    Integer(i128),
    Str(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PrefixExpression {
    pub operator: UnaryOp,
    pub rhs: Expression,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InfixExpression {
    pub lhs: Expression,
    pub operator: BinaryOp,
    pub rhs: Expression,
}

// This is an infix expression with 'as' as the binary operator
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CastExpression {
    pub lhs: Expression,
    pub r#type: Type,
}

#[derive(Debug, PartialEq, Eq, Clone)]
// Function definition
// fn add(x, y) {x+y}
pub struct FunctionDefinition {
    pub name: Ident,
    pub attribute : Option<Attribute>, // XXX: Currently we only have one attribute defined. If more attributes are needed per function, we can make this a vector and make attribute definition more expressive
    pub parameters: Vec<(Ident, Type)>,
    pub body: BlockStatement,
    pub return_type : Type,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrayLiteral {
    pub length: u128, // XXX: Maybe allow field element, so that the user can define the length using a constant
    pub r#type: Type,
    pub contents: Vec<Expression>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CallExpression {
    pub func_name: Ident,
    pub arguments: Vec<Expression>,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IndexExpression {
    pub collection_name: Ident, // XXX: For now, this will be the name of the array, as we do not support other collections
    pub index: Expression, // XXX: We accept two types of indices, either a normal integer or a constant
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NoirPath {
    Current,
    External(Vec<Ident>) // These are used for functions, and maybe constants in the future. Example: std::hash -> vec!["std", "hash"]
}

impl From<Vec<Ident>> for NoirPath {
    fn from(path: Vec<Ident>) -> NoirPath {
        if path.len() == 0 {
            NoirPath::Current
        } else {
            NoirPath::External(path)
        }
    }
}

impl Into<Vec<Ident>> for NoirPath {
    fn into(self) -> Vec<Ident> {
        match self {
            NoirPath::Current => Vec::new(),
            NoirPath::External(path) => path
        }
    }
}

impl NoirPath {
    pub fn to_string(&self) -> String {
        let mut string = String::new();

        match self {
            NoirPath::Current => return string,
            NoirPath::External(path) => {
                for ns in path.iter() {
                    string.push_str(&ns.0.contents);
                    string.push_str("::");
                }
                // Remove last `::`
                string.remove(string.len() - 1);
                string.remove(string.len() - 1);
            }
        }

        string
    }
    pub fn len(&self) -> usize {
        match self {
            NoirPath::Current => 0,
            NoirPath::External(path) => path.len()
        }
    }
    pub fn split_first(&self) -> Option<(&Ident, NoirPath)> {

        let path = match self {
            NoirPath::Current => return None,
            NoirPath::External(path) => path
        };

        if let Some((first, rest) ) = path.split_first() {
            return Some((first, rest.to_vec().into()))
        } else {
            return None
        }
    }
}
