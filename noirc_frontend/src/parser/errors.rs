use crate::lexer::errors::LexerError;
use crate::lexer::token::{Token, TokenKind, SpannedToken};

use thiserror::Error;
use noirc_errors::CustomDiagnostic as Diagnostic;
use noirc_errors::DiagnosableError;
use noirc_errors::Span;

#[derive(Error, Debug)]
pub enum ParserErrorKind {
    #[error("Lexer error found")]
    LexerError(LexerError),
    #[error(" expected expression, found `{}`", lexeme)]
    ExpectedExpression { span: Span, lexeme: String },
    #[error(" `{:?}` cannot be used as a binary operator.", lexeme)]
    NoInfixFunction { span: Span, lexeme: String },
    #[error("Unexpected token found")]
    UnexpectedToken { span: Span, expected: Token, found : Token },
    #[error("Unexpected token kind found")]
    UnexpectedTokenKind { span: Span, expected: TokenKind, found : TokenKind },
    #[error("Unstructured Error")]
    UnstructuredError { span: Span, message : String},
    #[error("Token is not a unary operation")]
    TokenNotUnaryOp { spanned_token: SpannedToken},
    #[error("Token is not a binary operation")]
    TokenNotBinaryOp { spanned_token: SpannedToken},
    #[error("Internal Compiler Error, unrecoverable")] // Actually lets separate these two types of errors
    InternalError{message : String, span : Span},
}

impl ParserErrorKind {
    pub fn into_err(self, file_id : usize) -> ParserError {
        ParserError {
            kind: self,
            file_id,
        }
    }
}

#[derive(Debug)]
pub struct ParserError {
    pub(crate) kind : ParserErrorKind,
    file_id : usize,
}


impl DiagnosableError for ParserError {
    fn to_diagnostic(&self) -> Diagnostic{
        match &self.kind {
            ParserErrorKind::LexerError(lex_err) => lex_err.to_diagnostic(),
            ParserErrorKind::InternalError{message, span} => unreachable!("Internal Error. This is a bug in the compiler. Please report the following message :\n {} \n with the following span {:?}", message,span),
            ParserErrorKind::ExpectedExpression{span, lexeme} => {
                let mut diag = Diagnostic::simple_error(format!("Unexpected start of an expression {}", lexeme), format!("did not expect this token"), *span);
                diag.add_note(format!("This error is commonly caused by either a previous error cascading or an unclosed delimiter."));
                diag
            },
            ParserErrorKind::NoInfixFunction{span, lexeme} => {
                Diagnostic::simple_error(format!("Token {} cannot be used as an Infix operator", lexeme), format!("cannot be used as a infix operator."), *span)
            },
            ParserErrorKind::TokenNotUnaryOp{spanned_token} => {
                Diagnostic::simple_error(format!("Unsupported unary operation {}", spanned_token.token()), format!("cannot use as a unary operation."), spanned_token.into_span())
            },
            ParserErrorKind::TokenNotBinaryOp{spanned_token} => {
                Diagnostic::simple_error(format!("Unsupported binary operation {}", spanned_token.token()), format!("cannot use as a binary operation."), spanned_token.into_span())
            },
            ParserErrorKind::UnexpectedToken{span , expected, found} => {
                Diagnostic::simple_error(format!("Expected a {} but found {}", expected, found), format!("Expected {}", expected), *span)
            }
            ParserErrorKind::UnexpectedTokenKind{span , expected, found} => {
                Diagnostic::simple_error(format!("Expected a {} but found {}", expected, found), format!("Expected {}", expected), *span)
            },
            ParserErrorKind::UnstructuredError{span, message} => {
                Diagnostic::simple_error("".to_owned(), message.to_string(), *span)
            },
        }
    }
}