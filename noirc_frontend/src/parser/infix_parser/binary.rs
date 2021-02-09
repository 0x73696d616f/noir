use super::*;
use crate::lexer::token::SpannedToken;
use noirc_errors::Spanned;
pub struct BinaryParser;

impl BinaryParser {
    /// Parses all expressions containing binary operations
    ///
    /// EXPR_LHS OP EXPR_RHS
    ///
    /// Cursor Start : `OP`
    ///
    /// Cursor End : `EXPR_RHS`
    pub fn parse(parser: &mut Parser, lhs: Expression) -> ParserExprKindResult {
        let operator = token_to_binary_op(&parser.curr_token, parser.file_id)?;

        // Check if the operator is a predicate
        // so that we can eagerly wrap it as a Predicate expression
        let is_predicate_op = operator.contents.is_comparator();

        // Parse rhs, precedence is extracted so that the
        // expression is grouped correctly
        let curr_precedence = Precedence::from(&parser.curr_token);
        parser.advance_tokens();
        let rhs = parser.parse_expression(curr_precedence)?;

        let infix_expression = Box::new(InfixExpression {
            lhs: lhs,
            operator,
            rhs: rhs.clone(),
        });

        if is_predicate_op {
            return Ok(ExpressionKind::Predicate(infix_expression));
        }
        return Ok(ExpressionKind::Infix(infix_expression));
    }
}
fn token_to_binary_op(spanned_tok: &SpannedToken, file_id: usize) -> Result<BinaryOp, ParserError> {
    let bin_op_kind: Option<BinaryOpKind> = spanned_tok.token().into();
    let bin_op_kind = bin_op_kind.ok_or(
        ParserErrorKind::TokenNotBinaryOp {
            spanned_token: spanned_tok.clone(),
        }
        .into_err(file_id),
    )?;
    Ok(Spanned::from(spanned_tok.into_span(), bin_op_kind))
}

#[cfg(test)]
mod test {

    use crate::{parser::test_parse, token::Token, Expression};

    fn dummy_expr() -> Expression {
        use crate::parser::prefix_parser::PrefixParser;
        const SRC: &'static str = r#"
            5;
        "#;
        let mut parser = test_parse(SRC);
        PrefixParser::Literal.parse(&mut parser).unwrap()
    }

    use super::BinaryParser;

    #[test]
    fn valid_syntax() {
        let vectors = vec![" + 6", " - k", " + (x + a)", " * (x + a) + (x - 4)"];

        for src in vectors {
            let mut parser = test_parse(src);
            let _ = BinaryParser::parse(&mut parser, dummy_expr()).unwrap();
        }
        // let end = parser.curr_token.clone();
        // let start = parser.curr_token.clone();
    }
    #[test]
    fn invalid_syntax() {
        let vectors = vec!["! x"];

        for src in vectors {
            let mut parser = test_parse(src);
            let _ = BinaryParser::parse(&mut parser, dummy_expr()).unwrap_err();
        }
    }
}
