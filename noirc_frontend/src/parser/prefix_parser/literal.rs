use super::*;

/// The LiteralParser specifies how we will parse all Literal Tokens
/// Except for function literals
pub struct LiteralParser;

impl PrefixParser for LiteralParser {
    /// Parses a Literal token
    fn parse(parser: &mut Parser) -> Expression {
        match parser.curr_token.clone().into() {
            Token::Int(x) => Expression::Literal(Literal::Integer(x)),
            Token::Str(x) => Expression::Literal(Literal::Str(x)),
            Token::Bool(x) => Expression::Literal(Literal::Bool(x)),
            Token::IntType(x) => Expression::Literal(Literal::Type(Type::from(&x))),
            x => panic!("expected a literal token, but found {}", x.to_string()),
        }
    }
}
