use super::{Precedence, Program};
use crate::ast::{BlockStatement, Expression, Statement, Type, ArraySize, ExpressionKind};
use crate::lexer::Lexer;
use crate::token::{Keyword, Token, TokenKind, SpannedToken};
use super::errors::ParserError;

use super::prefix_parser::PrefixParser;
use super::infix_parser::InfixParser;

pub type ParserResult<T> = Result<T, ParserError>;
pub type ParserExprKindResult = ParserResult<ExpressionKind>;
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
        Parser::new(Lexer::new(0,input))
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
        //
        // Its also usually bad UX to only show one error at a time.
    }
    
    // peaks at the next token
    // asserts that it should be of a certain variant
    // If it is, the parser is advanced
    pub(crate) fn peek_check_variant_advance(&mut self, token: &Token) -> Result<(), ParserError> {
        let same_variant = self.peek_token.is_variant(token);

        if !same_variant {
            let peeked_span = self.peek_token.into_span();
            let peeked_token = self.peek_token.token().clone();
            self.advance_tokens(); // We advance the token regardless, so the parser does not choke on a prefix function
            return Err(ParserError::UnexpectedToken{span : peeked_span, expected : token.clone(),found : peeked_token });
        }
        self.advance_tokens();
        return Ok(());
    }

    // peaks at the next token
    // asserts that it should be of a certain kind
    // If it is, the parser is advanced
    pub(crate) fn peek_check_kind_advance(&mut self, token_kind: TokenKind) -> Result<(), ParserError> {
        let peeked_kind = self.peek_token.kind();
        let same_kind = peeked_kind == token_kind;
        if !same_kind {
            let peeked_span = self.peek_token.into_span();
            self.advance_tokens();
            return Err(ParserError::UnexpectedTokenKind{span : peeked_span, expected : token_kind,found : peeked_kind })
        }
        self.advance_tokens();
        return Ok(());
    }

    /// A Program corresponds to a single module
    pub fn parse_program(&mut self) -> Result<Program, &Vec<ParserError>> {
        use super::prefix_parser::{FuncParser, UseParser, ModuleParser};

        let mut program = Program::with_capacity(self.lexer.by_ref().approx_len());

        while self.curr_token != Token::EOF {
            match self.curr_token.clone().into() {
                Token::Attribute(attr) => {
                    self.advance_tokens(); // Skip the attribute
                    let func_def = FuncParser::parse_fn_definition(self, Some(attr));
                    self.on_value(func_def, |value|program.push_function(value));
                },
                Token::Keyword(Keyword::Fn) => {
                    let func_def = FuncParser::parse_fn_definition(self, None);
                    self.on_value(func_def, |value|program.push_function(value));
                }
                Token::Keyword(Keyword::Mod) => {
                    let parsed_mod = ModuleParser::parse_module_decl(self);
                    self.on_value(parsed_mod, |module_identifier|program.push_module_decl(module_identifier));
                }
                Token::Keyword(Keyword::Use) => {
                    let import_stmt = UseParser::parse(self);
                    self.on_value(import_stmt, |value|program.push_import(value));
                }
                Token::Comment(_) => {
                    // This is a comment outside of a function.
                    // Currently we do nothing with Comment tokens
                    // It may be possible to store them in the AST, but this may not be helpful
                    // XXX: Maybe we can follow Rust and say by default all public functions need documentation?
                }
                tok => {
                    // XXX: We may allow global constants. We can use a subenum to remove the wildcard pattern
                    unreachable!(tok)
                }
            }
            // The current token will be the ending token for whichever branch was just picked
            // so we advance from that
            self.advance_tokens();
        }
        if self.errors.len() > 0 {
            return Err(&self.errors)
        } else {
            return Ok(program)
        }
    }

    fn on_value<T, F>(&mut self, parser_res : ParserResult<T>, mut func : F) 
            where F: FnMut(T) 
    {
        match parser_res {
            Ok(value) => func(value),
            Err(err) => {
                self.errors.push(err);
                self.synchronise();
            }
        }
    }

    // For now the synchonisation strategy is basic
    fn synchronise(&mut self) {
        loop {
            
            if self.peek_token ==  Token::EOF
            {
                break
            } 

            if self.choose_prefix_parser().is_some() {
                self.advance_tokens();
                break
            }

            if self.peek_token ==  Token::Keyword(Keyword::Private) || 
            self.peek_token ==  Token::Keyword(Keyword::Let) || 
            self.peek_token ==  Token::Keyword(Keyword::Fn) {
                self.advance_tokens();
                break
            }
            if self.peek_token ==  Token::RightBrace ||
            self.peek_token ==  Token::Semicolon
            
            {
                self.advance_tokens();
                self.advance_tokens();
                break
            } 
            
            self.advance_tokens()    
        }
    }

    pub fn parse_statement(&mut self) -> ParserStmtResult {
        use crate::parser::prefix_parser::{DeclarationParser,ConstrainParser};

        // The first type of statement we could have is a variable declaration statement
        if self.curr_token.can_start_declaration() {
            return DeclarationParser::parse_declaration_statement(self);
        };

        let stmt = match self.curr_token.token() {
            tk if tk.is_comment() => {
                // Comments here are within a function
                self.advance_tokens();
                return self.parse_statement()
            }
            Token::Keyword(Keyword::Constrain) => {
                Statement::Constrain(ConstrainParser::parse_constrain_statement(self)?)
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
            Token::Keyword(Keyword::If) => Some(PrefixParser::If),
            Token::Keyword(Keyword::For) => Some(PrefixParser::For),
            Token::LeftBracket => Some(PrefixParser::Array),
            x if x.kind() == TokenKind::Ident => Some(PrefixParser::Name),
            x if x.kind() == TokenKind::Literal => Some(PrefixParser::Literal),
            Token::Bang | Token::Minus => Some(PrefixParser::Unary),
            Token::LeftParen => Some(PrefixParser::Group),
            _ => None,
        }
    }
    fn choose_infix_parser(&self) -> Option<InfixParser> {
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
            | Token::NotEqual => Some(InfixParser::Binary),
            Token::Keyword(Keyword::As) => Some(InfixParser::Cast),
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
            return Err(ParserError::UnstructuredError{message : format!("Expected a }} to end the block statement"), span : self.curr_token.into_span()});
        }

        Ok(BlockStatement(statements))
    }

    pub(crate) fn parse_comma_separated_argument_list(
        &mut self,
        delimeter: Token,
    ) -> Result<Vec<Expression>, ParserError> {
        if self.peek_token == delimeter {
            self.advance_tokens();
            return Ok(Vec::new());
        }
        let mut arguments: Vec<Expression> = Vec::new();

        self.advance_tokens();
        arguments.push(self.parse_expression(Precedence::Lowest)?);
        while self.peek_token == Token::Comma {
            self.advance_tokens();
            self.advance_tokens();

            arguments.push(self.parse_expression(Precedence::Lowest)?);
        }

        self.peek_check_variant_advance(&delimeter)?;

        Ok(arguments)
    }

    // Parse Types
    pub(crate) fn parse_type(&mut self) -> Result<Type, ParserError> {
        // Currently we only support the default types and integers.
        // If we get into this function, then the user is specifying a type
        match self.curr_token.token() {
            Token::Keyword(Keyword::Witness) => Ok(Type::Witness),
            Token::Keyword(Keyword::Public) => Ok(Type::Public),
            Token::Keyword(Keyword::Constant) => Ok(Type::Constant),
            Token::Keyword(Keyword::Field) => Ok(Type::FieldElement),
            Token::IntType(int_type) => Ok(int_type.into()),
            Token::LeftBracket => self.parse_array_type(),
            k => {
                let message = format!("Expected a type, found {}", k);
                return Err(ParserError::UnstructuredError{message, span : self.curr_token.into_span()});
            },
        }
    }
    
    fn parse_array_type(&mut self) -> Result<Type, ParserError> {
        // Expression is of the form [3]Type
    
        // Current token is '['
        //
        // Next token should be an Integer or right brace
        let array_len = match self.peek_token.clone().into() {
            Token::Int(integer) => {
                
                if !integer.fits_in_u128() {
                    let message = format!("Array sizes must fit within a u128");
                    return Err(ParserError::UnstructuredError{message, span: self.peek_token.into_span()});

                }
                self.advance_tokens();
                ArraySize::Fixed(integer.to_u128())
            },
            Token::RightBracket => ArraySize::Variable,
            _ => {
                let message = format!("The array size is defined as [k] for fixed size or [] for variable length. k must be a literal");
                return Err(ParserError::UnstructuredError{message, span: self.peek_token.into_span()});
            },
        };

        self.peek_check_variant_advance(&Token::RightBracket)?;
    
        // Skip Right bracket
        self.advance_tokens();
    
        // Disallow [4][3]Witness ie Matrices
        if self.peek_token == Token::LeftBracket {
           return Err(ParserError::UnstructuredError{message  : format!("Currently Multi-dimensional arrays are not supported"), span : self.peek_token.into_span()})
        }
    
        let array_type = self.parse_type()?;
    
        Ok(Type::Array(array_len, Box::new(array_type)))
    }
}