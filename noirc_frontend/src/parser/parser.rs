use super::{Precedence, Program};
use crate::ast::{BlockStatement, Expression, Statement, Type, ArraySize};
use crate::lexer::Lexer;
use crate::token::{Keyword, Token, TokenKind, SpannedToken};
use super::errors::ParserError;

use super::prefix_parser::PrefixParser;
use super::infix_parser::InfixParser;

pub type ParserResult<T> = Result<T, ParserError>;
pub type ParserExprResult = ParserResult<Expression>;
type ParserStmtResult = ParserResult<Statement>;

// XXX: We can probably abstract the lexer away, as we really only need an Iterator of Tokens/ TokenStream
// XXX: Alternatively can make Lexer take a Reader, but will need to do a Bytes -> to char conversion. Can just return an error if cannot do conversion
// As this should not be leaked to any other part of the lib
pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) curr_token: SpannedToken,
    pub(crate) peek_token: SpannedToken,
    pub(crate) errors: Vec<ParserError>,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let curr_token = lexer.next_token().unwrap();
        let peek_token = lexer.next_token().unwrap();
        Parser {
            lexer,
            curr_token,
            peek_token,
            errors: Vec::new(),
        }
    }

    pub fn with_input(input : &'a str) -> Self {
        Parser::new(Lexer::new(input))
    }

    /// Note that this function does not alert the user of an EOF
    /// calling this function repeatedly will repeatedly give you 
    /// an EOF token. EOF tokens are not errors
    pub(crate) fn advance_tokens(&mut self) {
        self.curr_token = self.peek_token.clone();

        loop {
            match self.lexer.next_token() {
                Ok(spanned_token) => {
                    self.peek_token = spanned_token;
                    break;
                },
                Err(lex_err) => {
                    self.errors.push(ParserError::LexerError(lex_err))
                }        
            }
        }

        // At this point, we could check for lexer errors
        // and abort, however, if we do not we may be able to 
        // recover if the next token is the correct one
    }
    // peaks at the next token
    // asserts that it should be of a certain variant
    // If it is, the parser is advanced
    pub(crate) fn peek_check_variant_advance(&mut self, token: &Token) -> bool {
        let same_variant = self.peek_token.is_variant(token);

        if same_variant {
            self.advance_tokens();
            return true;
        }
        return false;
    }
    // peaks at the next token
    // asserts that it should be of a certain kind
    // If it is, the parser is advanced
    pub(crate) fn peek_check_kind_advance(&mut self, token_kind: TokenKind) -> bool {
        let same_kind = self.peek_token.kind() == token_kind;
        if same_kind {
            self.advance_tokens();
            return true;
        }
        return false;
    }

    // A program can contain many modules which themselves are programs
    pub fn parse_program(&mut self) -> Program {
        self.parse_unit(Token::EOF)
    }
    fn parse_unit(&mut self, delimeter : Token) -> Program {
        use super::prefix_parser::{FuncParser, UseParser, ModuleParser};

        let mut program = Program::with_capacity(self.lexer.by_ref().approx_len());

        while self.curr_token != delimeter {
            // First check if we have a function definition.
            // Function definitions are not added to the AST
            // Although we can have function literals starting with the function keyword
            // they will be self-contained within another function and they will start with a `let` token
            // Eg let add = fn(x,y) {x+y}
            match self.curr_token.clone().into() {
                Token::Attribute(attr) => {
                    self.advance_tokens(); // Skip the attribute
                    let func_def = FuncParser::parse_fn_definition(self, Some(attr));
                    self.on_value(func_def, |value|program.push_function(value));
                },
                Token::Keyword(Keyword::Fn) => {
                    let func_def = FuncParser::parse_fn_definition(self, None);
                    self.on_value(func_def, |value|program.push_function(value))
                }
                Token::Keyword(Keyword::Mod) => {
                   let (module_identifier, module) = ModuleParser::parse_module_definition(self);
                   program.push_module(module_identifier, module);
                }
                Token::Keyword(Keyword::Use) => {
                    let import_stmt = UseParser::parse(self);
                    program.push_import(import_stmt);
                }
                Token::Comment(_) => {
                    // This is a comment outside of a function.
                    // Currently we do nothing with Comment tokens
                    // It may be possible to store them in the AST, but this may not be helpful
                    // XXX: Maybe we can follow Rust and say by default all public functions need documentation?
                }
                _ => {
                    // Parse regular statements
                    let statement = self.parse_statement();
                    match statement {
                        Ok(stmt) => program.push_statement(stmt),
                        Err(err) => self.errors.push(err) 
                    };
                }
            }
            // The current token will be the ending token for whichever branch was just picked
            // so we advance from that
            self.advance_tokens();
        }

        program
    }

    fn on_value<T, F>(&mut self, parser_res : ParserResult<T>, mut func : F) 
            where F: FnMut(T) 
    {
        match parser_res {
            Ok(value) => func(value),
            Err(err) => self.errors.push(err)
        }
    }
    pub fn parse_module(&mut self) -> Program{
        self.parse_unit(Token::RightBrace)
    }
    pub fn parse_statement(&mut self) -> ParserStmtResult {
        use crate::parser::constraint_parser::ConstraintParser;
        use crate::parser::prefix_parser::{DeclarationParser, IfParser};

        // The first type of statement we could have is a variable declaration statement
        if self.curr_token.can_start_declaration() {
            return Ok(DeclarationParser::parse_declaration_statement(self, &self.curr_token.clone().into()));
        };

        let stmt = match self.curr_token.token() {
            tk if tk.is_comment() => {
                // Comments here are within a function
                self.advance_tokens();
                return self.parse_statement()
            }
            Token::Keyword(Keyword::Constrain) => {
                Statement::Constrain(ConstraintParser::parse_constrain_statement(self))
            }
            Token::Keyword(Keyword::If) => {
                Statement::If(IfParser::parse_if_statement(self)?)
            }
            _ => {
                let expr = self.parse_expression_statement()?;
                Statement::Expression(expr)
            }
        };
        // Check if the next token is a semi-colon(optional)
        if self.peek_token == Token::Semicolon {
            self.advance_tokens();
        };
        return Ok(stmt);
    }

    fn parse_expression_statement(&mut self) -> ParserExprResult {
        self.parse_expression(Precedence::Lowest)
    }

    pub(crate) fn parse_expression(&mut self, precedence: Precedence) -> ParserExprResult {
        // Calling this method means that we are at the beginning of a local expression
        // We may be in the middle of a global expression, but this does not matter
        let mut left_exp = match self.choose_prefix_parser() {
            Some(prefix_parser) => prefix_parser.parse(self)?,
            None => {
                return Err(ParserError::NoPrefixFunction{span : self.curr_token.into_span(), lexeme: self.curr_token.token().to_string()})
            }
        };

        while (self.peek_token != Token::Semicolon)
            && (precedence < Precedence::from(self.peek_token.token()))
        {
            match self.choose_infix_parser() {
                None => {
                    dbg!("No infix function found for {}", self.curr_token.token());
                    return Ok(left_exp.clone());
                }
                Some(infix_parser) => {
                    self.advance_tokens();
                    left_exp = infix_parser.parse(self, left_exp)?;
                }
            }
        }

        return Ok(left_exp);
    }
    fn choose_prefix_parser(&self) -> Option<PrefixParser> {
  
        match self.curr_token.token() {
            Token::Keyword(Keyword::For) => Some(PrefixParser::For),
            Token::LeftBracket => Some(PrefixParser::Array),
            x if x.kind() == TokenKind::Ident => Some(PrefixParser::Name),
            x if x.kind() == TokenKind::Literal => Some(PrefixParser::Literal),
            Token::Bang | Token::Minus => Some(PrefixParser::Unary),
            Token::LeftParen => Some(PrefixParser::Group),
            _ => None,
        }
    }
    fn choose_infix_parser(&mut self) -> Option<InfixParser> {
        match self.peek_token.token() {
            Token::Plus
            | Token::Minus
            | Token::Slash
            | Token::Pipe
            | Token::Ampersand
            | Token::Caret
            | Token::Star
            | Token::Less
            | Token::LessEqual
            | Token::Greater
            | Token::GreaterEqual
            | Token::Equal
            | Token::Assign
            | Token::Keyword(Keyword::As)
            | Token::NotEqual => Some(InfixParser::Binary),
            Token::LeftParen => Some(InfixParser::Call),
            Token::LeftBracket => Some(InfixParser::Index),
            Token::DoubleColon => Some(InfixParser::Path),
            _ => None,
        }
    }

    pub(crate) fn parse_block_statement(&mut self) -> Result<BlockStatement, ParserError> {
        let mut statements: Vec<Statement> = Vec::new();
        
        // Advance past the current token which is the left brace which was used to start the block statement
        // XXX: Check consistency with for parser, if parser and func parser
        self.advance_tokens();

        while (self.curr_token != Token::RightBrace) && (self.curr_token != Token::EOF) {
            statements.push(self.parse_statement()?);
            self.advance_tokens();
        }

        if self.curr_token != Token::RightBrace {
            panic!("Expected a } to end the block statement")
        }

        Ok(BlockStatement(statements))
    }

    pub(crate) fn parse_comma_separated_argument_list(
        &mut self,
        delimeter: Token,
    ) -> Vec<Expression> {
        if self.peek_token == delimeter {
            self.advance_tokens();
            return Vec::new();
        }
        let mut arguments: Vec<Expression> = Vec::new();

        self.advance_tokens();
        arguments.push(self.parse_expression(Precedence::Lowest).unwrap());
        while self.peek_token == Token::Comma {
            self.advance_tokens();
            self.advance_tokens();

            arguments.push(self.parse_expression(Precedence::Lowest).unwrap());
        }

        if !self.peek_check_variant_advance(&delimeter) {
            panic!("Expected a {} to end the list of arguments", delimeter)
        };

        arguments
    }

    // Parse Types
    pub(crate) fn parse_type(&mut self) -> Type {
        // Currently we only support the default types and integers.
        // If we get into this function, then the user is specifying a type
        match self.curr_token.token() {
            Token::Keyword(Keyword::Witness) => Type::Witness,
            Token::Keyword(Keyword::Public) => Type::Public,
            Token::Keyword(Keyword::Constant) => Type::Constant,
            Token::Keyword(Keyword::Field) => Type::FieldElement,
            Token::IntType(int_type) => int_type.into(),
            Token::LeftBracket => self.parse_array_type(),
            k => unimplemented!("This type is currently not supported, `{}`", k),
        }
    }
    
    fn parse_array_type(&mut self) -> Type {
        // Expression is of the form [3]Type
    
        // Current token is '['
        //
        // Next token should be an Integer or right brace
        let array_len = match self.peek_token.clone().into() {
            Token::Int(integer) => {
                if integer < 0 {
                    panic!("Cannot have a negative array size, [-k]Type is disallowed")
                }
                self.advance_tokens();
                ArraySize::Fixed(integer as u128)
            },
            Token::RightBracket => ArraySize::Variable,
            _ => panic!("The array size is defined as [k] for fixed size or [] for variable length"),
        };

        if !self.peek_check_variant_advance(&Token::RightBracket) {
            panic!(
                "expected a `]` after integer, got {}",
                self.peek_token.token()
            )
        }
    
        // Skip Right bracket
        self.advance_tokens();
    
        // Disallow [4][3]Witness ie Matrices
        if self.peek_token == Token::LeftBracket {
            panic!("Currently Multi-dimensional arrays are not supported")
        }
    
        let array_type = self.parse_type();
    
        Type::Array(array_len, Box::new(array_type))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::{
        BlockStatement, CallExpression, Expression, FunctionDefinition,
         Ident, IfStatement, InfixExpression, Literal, PrefixExpression,
        Statement, Type,
    };
    #[test]
    fn test_basic_let() {
        // XXX: Incomplete, as we do not check the expression
        let input = "
            let x = 5;
            let y = 15;
            let z = 20;
        ";

        let test_iden = vec!["x", "y", "z"];

        let mut parser = Parser::new(Lexer::new(input));

        let program = parser.parse_program();
        for (stmt, iden) in program.statements.iter().zip(test_iden.iter()) {
            helper_test_let(stmt, iden);
        }

        assert_eq!(program.statements.len(), 3);
    }

    fn helper_test_let(statement: &Statement, iden: &str) {
        // First make sure that the statement is a let statement
        let let_stmt = match statement {
            Statement::Let(stmt) => stmt,
            _ => panic!("Expected a let statement"),
        };

        // Now assert the correct identifier is in the let statement
        assert_eq!(let_stmt.identifier.0, iden);
    }

    #[test]
    fn test_parse_identifier() {
        let input = "hello;world;This_is_a_word";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let test_iden = vec!["hello", "world", "This_is_a_word"];

        for (stmt, iden) in program.statements.into_iter().zip(test_iden.iter()) {
            // Cast to an expression
            let expression = match stmt {
                Statement::Expression(x) => x,
                _ => unreachable!(),
            };
            // Extract the identifier
            let name = match expression {
                Expression::Ident(x) => x,
                _ => unreachable!(),
            };

            assert_eq!(iden, &name)
        }
    }

    #[test]
    fn test_parse_literals() {
        let input = "10;true;\"string_literal\"";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let test_iden = vec![
            Literal::Integer(10),
            Literal::Bool(true),
            Literal::Str("string_literal".to_string()),
        ];

        for (stmt, expected_lit) in program.statements.into_iter().zip(test_iden.iter()) {
            // Cast to an expression
            let expression = match stmt {
                Statement::Expression(x) => x,
                _ => unreachable!(),
            };
            // Extract the literal
            let literal = match expression {
                Expression::Literal(x) => x,
                _ => unreachable!(),
            };

            assert_eq!(expected_lit, &literal)
        }
    }
    #[test]
    fn test_parse_prefix() {
        use crate::ast::*;
        let input = "!99;-100;!true";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let test_iden = vec![
            PrefixExpression {
                operator: UnaryOp::Not,
                rhs: Expression::Literal(Literal::Integer(99)),
            },
            PrefixExpression {
                operator: UnaryOp::Minus,
                rhs: Expression::Literal(Literal::Integer(100)),
            },
            PrefixExpression {
                operator: UnaryOp::Not,
                rhs: Expression::Literal(Literal::Bool(true)),
            },
        ];

        for (stmt, expected_lit) in program.statements.into_iter().zip(test_iden.iter()) {
            // Cast to an expression
            let expression = match stmt {
                Statement::Expression(x) => x,
                _ => unreachable!(),
            };
            // Extract the prefix expression
            let literal = match expression {
                Expression::Prefix(x) => x,
                _ => unreachable!(),
            };

            assert_eq!(*expected_lit, *literal)
        }
    }

    #[test]
    fn test_parse_infix() {
        let input = "5+5;10*5;true == false; false != false";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let test_iden = vec![
            InfixExpression {
                lhs: Expression::Literal(Literal::Integer(5)),
                operator: Token::Plus.into(),
                rhs: Expression::Literal(Literal::Integer(5)),
            },
            InfixExpression {
                lhs: Expression::Literal(Literal::Integer(10)),
                operator: Token::Star.into(),
                rhs: Expression::Literal(Literal::Integer(5)),
            },
            InfixExpression {
                lhs: Expression::Literal(Literal::Bool(true)),
                operator: Token::Equal.into(),
                rhs: Expression::Literal(Literal::Bool(false)),
            },
            InfixExpression {
                lhs: Expression::Literal(Literal::Bool(false)),
                operator: Token::NotEqual.into(),
                rhs: Expression::Literal(Literal::Bool(false)),
            },
        ];

        for (stmt, expected_lit) in program.statements.into_iter().zip(test_iden.iter()) {
            // Cast to an expression
            let expression = match stmt {
                Statement::Expression(x) => x,
                _ => unreachable!(),
            };
            // Extract the infix expression
            let literal = match expression {
                Expression::Predicate(x) => x,
                Expression::Infix(x) => x,
                _ => unreachable!(),
            };

            assert_eq!(*expected_lit, *literal)
        }
    }
    #[test]
    fn test_parse_grouped() {
        use crate::ast::UnaryOp;

        let input = "-(5+10);-5+10";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        // Test the first expression : -(5+10)
        let grouped_expression = PrefixExpression {
            operator: UnaryOp::Minus,
            rhs: Expression::Infix(Box::new(InfixExpression {
                lhs: Expression::Literal(Literal::Integer(5)),
                operator: Token::Plus.into(),
                rhs: Expression::Literal(Literal::Integer(10)),
            })),
        };

        let stmt = program.statements[0].clone();
        let expected_lit = grouped_expression;
        // Cast to an expression
        let expression = match stmt {
            Statement::Expression(x) => x,
            _ => unreachable!(),
        };
        // Extract the prefix expression
        let prefix = match expression {
            Expression::Prefix(x) => x,
            _ => unreachable!(),
        };
        assert_eq!(*prefix, expected_lit);

        // Test the second expression : -5+10
        let ungrouped_expression = InfixExpression {
            lhs: Expression::Prefix(Box::new(PrefixExpression {
                operator: UnaryOp::Minus,
                rhs: Expression::Literal(Literal::Integer(5)),
            })),
            operator: Token::Plus.into(),
            rhs: Expression::Literal(Literal::Integer(10)),
        };

        let stmt = program.statements[1].clone();
        let expected_lit = ungrouped_expression;
        // Cast to an expression
        let expression = match stmt {
            Statement::Expression(x) => x,
            _ => unreachable!(),
        };
        // Extract the prefix expression
        let prefix = match expression {
            Expression::Infix(x) => x,
            _ => unreachable!(),
        };
        assert_eq!(*prefix, expected_lit);
    }

    #[test]
    fn test_parse_if_expression() {
        let input = "if (x < y) { x };";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let expected_if = IfStatement {
            condition: Expression::Predicate(Box::new(InfixExpression {
                lhs: Expression::Ident("x".to_string()),
                operator: Token::Less.into(),
                rhs: Expression::Ident("y".to_string()),
            })),
            consequence: BlockStatement(vec![Statement::Expression(
                Expression::Ident("x".to_string())
            )]),
            alternative: None,
        };

        let stmt = program.statements[0].clone();
        let if_stmt = match stmt {
            Statement::If(x) => x,
            _ => unreachable!(),
        };
        assert_eq!(*if_stmt, expected_if)
    }
    #[test]
    fn test_parse_if_else_expression() {
        let input = "if (foo < bar) { cat } else {dog};";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let expected_if = IfStatement {
            condition: Expression::Predicate(Box::new(InfixExpression {
                lhs: Expression::Ident("foo".to_string()),
                operator: Token::Less.into(),
                rhs: Expression::Ident("bar".to_string()),
            })),
            consequence: BlockStatement(vec![Statement::Expression(
                Expression::Ident("cat".to_string()),
            )]),
            alternative: Some(BlockStatement(vec![Statement::Expression(
                Expression::Ident("dog".to_string()),
            )])),
        };

        let stmt = program.statements[0].clone();
        let if_stmt = match stmt {
            Statement::If(x) => x,
            _ => unreachable!(),
        };

        assert_eq!(*if_stmt, expected_if)
    }

    #[test]
    fn test_parse_int_type() {
        use crate::parser::prefix_parser::DeclarationParser;
        use crate::ast::{PrivateStatement, Signedness};
        let input = "priv x : i102 = a";
        let mut parser = Parser::new(Lexer::new(input));
        let stmt = DeclarationParser::parse_declaration_statement(
            &mut parser,
            &Token::Keyword(Keyword::Private),
        );

        let priv_stmt_expected = PrivateStatement {
            identifier: Ident("x".into()),
            r#type: Type::Integer(Signedness::Signed, 102).into(),
            expression: Expression::Ident("a".into()),
        };
        match stmt {
            Statement::Private(priv_stmt) => {
                assert_eq!(priv_stmt, priv_stmt_expected);
            }
            _ => panic!("Expected a private statement"),
        }
    }

    #[test]
    // XXX: This just duplicates most of test_funct_literal. Refactor to avoid duplicate code
    fn test_parse_function_def_literal() {
        let input = "fn add(x : Public,y : Constant){x+y}";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let parameters = vec![
            (Ident("x".into()), Type::Public),
            (Ident("y".into()), Type::Constant),
        ];

        let infix_expression = InfixExpression {
            lhs: Expression::Ident("x".to_string()),
            operator: Token::Plus.into(),
            rhs: Expression::Ident("y".to_string()),
        };

        let expected = vec![FunctionDefinition {
            name: Ident("add".into()),
            attribute : None,
            parameters: parameters,
            body: BlockStatement(vec![Statement::Expression(
                Expression::Infix(Box::new(infix_expression)),
            )]),
            return_type : Type::Unit,        }];

        for (expected_def, got_def) in expected.into_iter().zip(program.functions.into_iter()) {
            assert_eq!(expected_def, got_def);
        }
    }

    #[test]
    fn test_parse_call_expression() {
        let input = "add(1,2+3)";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let test_iden = vec![CallExpression {
            func_name: Ident("add".to_string()),
            arguments: vec![
                Expression::Literal(Literal::Integer(1)),
                Expression::Infix(Box::new(InfixExpression {
                    lhs: Expression::Literal(Literal::Integer(2)),
                    operator: Token::Plus.into(),
                    rhs: Expression::Literal(Literal::Integer(3)),
                })),
            ],
        }];

        for (stmt, expected_lit) in program.statements.into_iter().zip(test_iden.iter()) {
            // Cast to an expression
            let expression = match stmt {
                Statement::Expression(x) => x,
                _ => unreachable!(),
            };
            // Extract the function literal expression
            let call_expr = match expression {
                Expression::Call(_,x) => x,
                _ => unreachable!(),
            };

            assert_eq!(*expected_lit, *call_expr)
        }
    }
}
