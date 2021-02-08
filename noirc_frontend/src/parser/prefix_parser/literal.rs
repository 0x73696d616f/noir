use super::*;

/// The LiteralParser specifies how we will parse all Literal Tokens
/// Except for function literals
pub struct LiteralParser;

impl LiteralParser {
    /// Parses a Literal token
    pub fn parse(parser: &mut Parser) -> ParserExprKindResult {
        let expr = match parser.curr_token.clone().into() {
            Token::Int(x) => ExpressionKind::Literal(Literal::Integer(x)),
            Token::Str(x) => ExpressionKind::Literal(Literal::Str(x)),
            Token::Bool(x) => ExpressionKind::Literal(Literal::Bool(x)),
            x => {
                return Err(ParserErrorKind::UnexpectedTokenKind {
                    span: parser.curr_token.into_span(),
                    expected: TokenKind::Literal,
                    found: x.kind(),
                }
                .into_err(parser.file_id))
            }
        };
        Ok(expr)
    }
}
