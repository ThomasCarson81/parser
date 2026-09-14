use std::env;
use std::fs;
use syntax::{
    ast::{Expression, Program, Statement, Token, TokenType},
    functions::FunctionParameter,
    types::{Number, Type, Value},
};

struct TokenStream {
    tokens: Vec<Token>,
    current: usize,
}
impl TokenStream {
    fn new(tokens: Vec<Token>) -> TokenStream {
        return TokenStream { tokens, current: 0 };
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }
    fn peek_by(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset)
    }
    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.current)?.clone();
        self.current += 1;
        Some(token)
    }
    fn is_at_end(&self) -> bool {
        self.peek()
            .map_or(true, |token| token.token_type == TokenType::EndOfFile)
    }
}

#[derive(Debug)]
enum ParserError {
    UnexpectedToken(Token, TokenType),
    UnexpectedTokenNoExpected(Token),
    InvalidTokenFormat(String),
    CouldNotReadFile(String),
    TokenStreamEnded,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        println!(
            "Please provide a file to parse, and optionally an output file to write the AST to."
        );
        return;
    }

    let mut tokens = TokenStream::new(get_tokens_from_file(&args[1]).unwrap_or_else(|e| {
        println!("Error reading tokens from file: {:?}", e);
        std::process::exit(1);
    }));

    let program = match parse_program(&mut tokens) {
        Ok(program) => program,
        Err(e) => {
            match e {
                ParserError::UnexpectedToken(got, expected) => {
                    println!(
                        "Unexpected token: {} at line {}, Expected: {}, got '{}'",
                        got.token_type, got.line, expected, got.lexeme
                    );
                }
                ParserError::UnexpectedTokenNoExpected(token) => {
                    println!("Unexpected token: {} on line {}", token.lexeme, token.line);
                }
                ParserError::InvalidTokenFormat(msg) => {
                    println!("Invalid token format: {}", msg);
                }
                ParserError::CouldNotReadFile(msg) => {
                    println!("Could not read file: {}", msg);
                }
                ParserError::TokenStreamEnded => {
                    println!("Token stream ended unexpectedly. (parsing)");
                }
            }
            std::process::exit(1);
        }
    };
    if args.len() == 3 {
        let json_result = serde_json::to_string_pretty(&program);
        match json_result {
            Ok(json) => {
                write_ast_to_file(&args[2], &json);
            }
            Err(e) => {
                println!("Error serialising AST JSON: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        for statement in program.statements {
            println!("{:?}", statement);
        }
    }
}

fn get_tokens_from_file(file_path: &str) -> Result<Vec<Token>, ParserError> {
    let file_content = std::fs::read_to_string(&file_path)
        .map_err(|e| ParserError::CouldNotReadFile(format!("Could not read file: {}", e)))?;
    let tokens = serde_json::from_str(&file_content)
        .map_err(|e| ParserError::CouldNotReadFile(e.to_string()))?;
    Ok(tokens)
}

fn write_ast_to_file(file_path: &str, ast: &str) {
    fs::write(file_path, ast).expect("Unable to write to file");
}

fn consume_token(tokens: &mut TokenStream, expected: TokenType) -> Result<Token, ParserError> {
    let peeked_token = tokens.peek().ok_or(ParserError::TokenStreamEnded)?.clone();
    if peeked_token.token_type != expected {
        return Err(ParserError::UnexpectedToken(peeked_token, expected));
    }
    tokens.advance();
    Ok(peeked_token)
}

fn check_token(tokens: &mut TokenStream, expected: TokenType) -> Result<bool, ParserError> {
    let peeked_token = tokens.peek().ok_or(ParserError::TokenStreamEnded)?;
    Ok(peeked_token.token_type == expected)
}

fn parse_program(tokens: &mut TokenStream) -> Result<Program, ParserError> {
    let mut statements: Vec<Statement> = Vec::new();
    while !tokens.is_at_end() {
        let statement = parse_statement(tokens, true)?;
        statements.push(statement);
    }
    Ok(Program { statements })
}

fn parse_statement(
    tokens: &mut TokenStream,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    let next_token = tokens.peek().ok_or(ParserError::TokenStreamEnded)?;
    let statement = match next_token.token_type {
        TokenType::Let => Ok(parse_declaration(tokens, consume_semicolon)?),
        TokenType::Identifier => Ok(parse_identifier_statement(tokens, consume_semicolon)?),
        TokenType::If => Ok(parse_if_statement(tokens)?),
        TokenType::Else => {
            // cannot have else tokens on their own
            Err(ParserError::UnexpectedTokenNoExpected(next_token.clone()))
        }
        TokenType::While => Ok(parse_while_statement(tokens)?),
        TokenType::For => Ok(parse_for_statement(tokens)?),
        TokenType::Fn => Ok(parse_function_definition(tokens)?),
        TokenType::Return => Ok(parse_return_statement(tokens)?),
        TokenType::Break => Ok(parse_break_statement(tokens)?),
        TokenType::Continue => Ok(parse_continue_statement(tokens)?),
        TokenType::Scope => Ok(parse_scope_statement(tokens)?),
        _ => {
            let expr = Statement::ExpressionStatement {
                expression: parse_expression(tokens)?,
            };
            if consume_semicolon {
                consume_token(tokens, TokenType::Semicolon)?;
            }
            Ok(expr)
        }
    };
    statement
}

fn parse_declaration(
    tokens: &mut TokenStream,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Let)?;
    let mutable = check_token(tokens, TokenType::Mut)?;

    if mutable {
        consume_token(tokens, TokenType::Mut)?;
    }

    let ident = consume_token(tokens, TokenType::Identifier)?;

    let mut var_type: Option<Type> = None;

    if check_token(tokens, TokenType::Colon)? {
        consume_token(tokens, TokenType::Colon)?;
        let type_token = consume_token(tokens, TokenType::TypeName)?;
        var_type = Some(Type::from_str(&type_token.lexeme).ok_or(
            ParserError::InvalidTokenFormat(format!(
                "Parser encounterd unknown type: '{}' on line {}",
                type_token.lexeme, type_token.line
            )),
        )?);
    }
    consume_token(tokens, TokenType::Equals)?;
    let expr = parse_expression(tokens)?;
    if consume_semicolon {
        consume_token(tokens, TokenType::Semicolon)?;
    }
    Ok(Statement::Let {
        identifier: ident.lexeme,
        expression: expr,
        mutable: mutable,
        var_type: var_type,
    })
}

fn parse_function_definition(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Fn)?;
    let ident = consume_token(tokens, TokenType::Identifier)?;
    consume_token(tokens, TokenType::LeftBracket)?;
    let mut params: Vec<FunctionParameter> = Vec::new();
    while !check_token(tokens, TokenType::RightBracket)? {
        let param = consume_token(tokens, TokenType::Identifier)?;
        consume_token(tokens, TokenType::Colon)?;
        let annotation = consume_token(tokens, TokenType::TypeName)?;
        let param_type = Type::from_str(&annotation.lexeme).ok_or(
            ParserError::InvalidTokenFormat(format!("unknown type: {}", &annotation.lexeme)),
        )?;
        params.push(FunctionParameter {
            name: param.lexeme,
            param_type,
        });
        if !check_token(tokens, TokenType::Comma)? {
            break;
        }
        consume_token(tokens, TokenType::Comma)?;
    }
    consume_token(tokens, TokenType::RightBracket)?;
    let return_type: Type;
    if check_token(tokens, TokenType::Colon)? {
        consume_token(tokens, TokenType::Colon)?;
        let annotation = consume_token(tokens, TokenType::TypeName)?;
        return_type = Type::from_str(&annotation.lexeme).ok_or(ParserError::InvalidTokenFormat(
            format!("unknown type: {}", &annotation.lexeme),
        ))?;
    } else {
        return_type = Type::Unit;
    }
    let body = parse_block(tokens)?;
    Ok(Statement::FunctionDefinition {
        identifier: ident.lexeme,
        parameters: params,
        body: body,
        return_type: return_type,
    })
}

fn parse_return_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Return)?;
    if check_token(tokens, TokenType::Semicolon)? {
        consume_token(tokens, TokenType::Semicolon)?;
        return Ok(Statement::Return { value: None });
    }
    let expr = parse_expression(tokens)?;
    consume_token(tokens, TokenType::Semicolon)?;
    Ok(Statement::Return { value: Some(expr) })
}

fn parse_break_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Break)?;
    consume_token(tokens, TokenType::Semicolon)?;
    Ok(Statement::Break)
}

fn parse_continue_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Continue)?;
    consume_token(tokens, TokenType::Semicolon)?;
    Ok(Statement::Continue)
}

fn parse_identifier_statement(
    tokens: &mut TokenStream,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    let next_token = tokens.peek_by(1).ok_or(ParserError::TokenStreamEnded)?;
    let statement = match next_token.token_type {
        TokenType::Equals  // a = b
        | TokenType::PlusEquals // a += b
        | TokenType::MinusEquals // a-= b
        | TokenType::AsteriskEquals // a *= b
        | TokenType::SlashEquals // a /= b
        | TokenType::ModuloEquals => // a %= b
            Ok(parse_assignment(tokens, consume_semicolon)?),
        TokenType::Increment // a++
        | TokenType::Decrement // a--
        | TokenType::LeftBracket
        | TokenType::Walrus
         => {
            let expr = parse_expression(tokens)?;
            if consume_semicolon {
                consume_token(tokens, TokenType::Semicolon)?;
            }
            Ok(Statement::ExpressionStatement {
                expression: expr,
            })
        }
        // TODO: a.b, a[b]
        _ => { // any other token, eg "a 4" is invalid
            let token: Token = Token {
                token_type: next_token.token_type,
                lexeme: next_token.lexeme.clone(),
                line: next_token.line,
            };
            Err(ParserError::UnexpectedTokenNoExpected(token))
        }
    }?;
    Ok(statement)
}

fn parse_assignment(
    tokens: &mut TokenStream,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    let ident = consume_token(tokens, TokenType::Identifier)?;
    let next_tok = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;
    let op = match next_tok.token_type {
        TokenType::Equals
        | TokenType::PlusEquals
        | TokenType::MinusEquals
        | TokenType::AsteriskEquals
        | TokenType::SlashEquals
        | TokenType::ModuloEquals => Ok(next_tok),
        _ => Err(ParserError::UnexpectedTokenNoExpected(next_tok)),
    }?;
    let mut expr = parse_expression(tokens)?;
    if consume_semicolon {
        consume_token(tokens, TokenType::Semicolon)?;
    }
    if op.token_type != TokenType::Equals {
        // unfold shorthand operators into assigment expressions
        // eg a += b => a = a + b
        let new_op = match op.token_type {
            TokenType::PlusEquals => Ok(TokenType::Plus),
            TokenType::MinusEquals => Ok(TokenType::Minus),
            TokenType::AsteriskEquals => Ok(TokenType::Asterisk),
            TokenType::SlashEquals => Ok(TokenType::Slash),
            TokenType::ModuloEquals => Ok(TokenType::Modulo),
            _ => Err(ParserError::UnexpectedTokenNoExpected(op.clone())),
        }?;
        expr = Expression::BinaryOp {
            left: Box::new(Expression::Identifier(ident.lexeme.clone())),
            operator: new_op,
            right: Box::new(expr),
            inferred_type: None,
        }
    }
    Ok(Statement::Assignment {
        identifier: ident.lexeme,
        expression: expr,
    })
}

fn parse_if_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::If)?;
    let condition = parse_expression(tokens)?;
    let body: Vec<Statement> = parse_block(tokens)?;
    let mut else_block: Option<Vec<Statement>> = None;
    if check_token(tokens, TokenType::Else)? {
        else_block = Some(parse_else_statement(tokens)?);
    }
    Ok(Statement::If {
        condition,
        body,
        else_block,
    })
}

fn parse_else_statement(tokens: &mut TokenStream) -> Result<Vec<Statement>, ParserError> {
    consume_token(tokens, TokenType::Else)?;
    // let body = parse_block(tokens)?;
    let mut body = Vec::new();
    if check_token(tokens, TokenType::If)? {
        body.push(parse_if_statement(tokens)?);
    } else {
        body.extend(parse_block(tokens)?);
    }
    Ok(body)
}

fn parse_while_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::While)?;
    let condition = parse_expression(tokens)?;
    let body = parse_block(tokens)?;
    Ok(Statement::While {
        condition: condition,
        body: body,
    })
}

fn parse_for_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::For)?;
    consume_token(tokens, TokenType::LeftBracket)?;
    let init = if check_token(tokens, TokenType::Let)? {
        Some(Box::new(parse_declaration(tokens, false)?))
    } else {
        None
    };
    consume_token(tokens, TokenType::Semicolon)?;
    let condition = if !check_token(tokens, TokenType::Semicolon)? {
        Some(parse_expression(tokens)?)
    } else {
        None
    };
    consume_token(tokens, TokenType::Semicolon)?;
    let increment = if !check_token(tokens, TokenType::RightBracket)? {
        Some(Box::new(parse_statement(tokens, false)?))
    } else {
        None
    };
    consume_token(tokens, TokenType::RightBracket)?;
    let body = parse_block(tokens)?;
    Ok(Statement::For {
        initialiser: init,
        condition,
        increment,
        body,
    })
}

fn parse_scope_statement(tokens: &mut TokenStream) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::Scope)?;
    let body = parse_block(tokens)?;
    Ok(Statement::Scope { body })
}

fn parse_expression(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    Ok(parse_walrus(tokens)?)
}

fn parse_walrus(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_logical_or(tokens)?;

    while check_token(tokens, TokenType::Walrus)? {
        consume_token(tokens, TokenType::Walrus)?;
        let right = parse_logical_or(tokens)?;

        match &expr {
            Expression::Identifier(_) => {
                expr = Expression::BinaryOp {
                    left: Box::new(expr),
                    operator: TokenType::Walrus,
                    right: Box::new(right),
                    inferred_type: None,
                }
            }
            _ => {
                return Err(ParserError::InvalidTokenFormat(String::from(
                    "Walrus currently only supported by identfiers.",
                )));
            }
        }
    }

    Ok(expr)
}

fn parse_logical_or(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_logical_and(tokens)?;

    while check_token(tokens, TokenType::LogicalOr)? {
        consume_token(tokens, TokenType::LogicalOr)?;
        let right = parse_logical_and(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: TokenType::LogicalOr,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_logical_and(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_equality(tokens)?;

    while check_token(tokens, TokenType::LogicalAnd)? {
        consume_token(tokens, TokenType::LogicalAnd)?;
        let right = parse_equality(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: TokenType::LogicalAnd,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_equality(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_comparison(tokens)?;

    while check_token(tokens, TokenType::EqualTo)? || check_token(tokens, TokenType::NotEqualTo)? {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;
        let right = parse_comparison(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_comparison(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_term(tokens)?;

    while check_token(tokens, TokenType::GreaterThan)?
        || check_token(tokens, TokenType::GreaterThanOrEqualTo)?
        || check_token(tokens, TokenType::LessThan)?
        || check_token(tokens, TokenType::LessThanOrEqualTo)?
    {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;
        let right = parse_term(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_term(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_factor(tokens)?;

    while check_token(tokens, TokenType::Plus)? || check_token(tokens, TokenType::Minus)? {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;
        let right = parse_factor(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_factor(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_unary_prefix(tokens)?;

    while check_token(tokens, TokenType::Asterisk)?
        || check_token(tokens, TokenType::Slash)?
        || check_token(tokens, TokenType::Modulo)?
    {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;
        let right = parse_unary_prefix(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
            inferred_type: None,
        };
    }

    Ok(expr)
}

fn parse_unary_prefix(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    if check_token(tokens, TokenType::Not)?
        || check_token(tokens, TokenType::Increment)?
        || check_token(tokens, TokenType::Decrement)?
        || check_token(tokens, TokenType::Minus)?
    {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;

        let operand = parse_unary_prefix(tokens)?;

        match (&operator.token_type, &operand) {
            (TokenType::Increment, Expression::Identifier(_))
            | (TokenType::Decrement, Expression::Identifier(_))
            | (TokenType::Minus, _)
            | (TokenType::Not, _) => {
                return Ok(Expression::UnaryOp {
                    operator: operator.token_type,
                    operand: Box::new(operand),
                    inferred_type: None,
                });
            }
            _ => {
                return Err(ParserError::InvalidTokenFormat(format!(
                    "Increment and decrement currently only supported by identfiers. {:?}, line {}",
                    operator, operator.line
                )));
            }
        };
    }

    parse_unary_postfix(tokens)
}

fn parse_unary_postfix(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let mut expr = parse_primary(tokens)?;

    while check_token(tokens, TokenType::Increment)?
        || check_token(tokens, TokenType::Decrement)?
        || check_token(tokens, TokenType::LeftBracket)?
    {
        let operator = tokens.advance().ok_or(ParserError::TokenStreamEnded)?;

        match operator.token_type {
            TokenType::Increment | TokenType::Decrement => match &expr {
                Expression::Identifier(_) => {
                    expr = Expression::UnaryOp {
                        operator: operator.token_type,
                        operand: Box::new(expr),
                        inferred_type: None,
                    }
                }
                _ => {
                    return Err(ParserError::InvalidTokenFormat(String::from(
                        "Increment and decrement currently only supported by identfiers.",
                    )));
                }
            },
            TokenType::LeftBracket => {
                let mut args: Vec<Expression> = Vec::new();
                while !check_token(tokens, TokenType::RightBracket)? {
                    args.push(parse_expression(tokens)?);
                    if !check_token(tokens, TokenType::Comma)? {
                        break;
                    }
                    consume_token(tokens, TokenType::Comma)?;
                }
                consume_token(tokens, TokenType::RightBracket)?;
                match expr {
                    Expression::Identifier(ident) => {
                        expr = Expression::UnresolvedFunctionCall {
                            identifier: ident,
                            arguments: args,
                        }
                    }
                    _ => {
                        return Err(ParserError::InvalidTokenFormat(String::from(
                            "Function calls currently only supported by identfiers.",
                        )));
                    }
                }
            }
            _ => {
                return Err(ParserError::UnexpectedTokenNoExpected(operator));
            }
        }
    }

    Ok(expr)
}

fn parse_primary(tokens: &mut TokenStream) -> Result<Expression, ParserError> {
    let next_token = tokens.peek().ok_or(ParserError::TokenStreamEnded)?;
    match next_token.token_type {
        TokenType::Number => {
            let tok = consume_token(tokens, TokenType::Number)?;
            let number = Expression::Literal(Value::Number(parse_number(&tok.lexeme)?));
            Ok(number)
        }
        TokenType::String => {
            let tok = consume_token(tokens, TokenType::String)?;
            let str_lit = Expression::Literal(Value::String(tok.lexeme));
            Ok(str_lit)
        }
        TokenType::True => {
            consume_token(tokens, TokenType::True)?;
            Ok(Expression::Literal(Value::Bool(true)))
        }
        TokenType::False => {
            consume_token(tokens, TokenType::False)?;
            Ok(Expression::Literal(Value::Bool(false)))
        }
        TokenType::Identifier => {
            let ident = consume_token(tokens, TokenType::Identifier)?;

            Ok(Expression::Identifier(ident.lexeme))
        }
        TokenType::LeftBracket => {
            consume_token(tokens, TokenType::LeftBracket)?;
            let expr = parse_expression(tokens)?;
            consume_token(tokens, TokenType::RightBracket)?;
            Ok(expr)
        }
        _ => Err(ParserError::UnexpectedTokenNoExpected(next_token.clone())),
    }
}

fn parse_block(tokens: &mut TokenStream) -> Result<Vec<Statement>, ParserError> {
    consume_token(tokens, TokenType::LeftSquiggly)?;

    let mut statements = Vec::new();

    while !check_token(tokens, TokenType::RightSquiggly)? {
        statements.push(parse_statement(tokens, true)?);
    }

    consume_token(tokens, TokenType::RightSquiggly)?;

    Ok(statements)
}

fn parse_number(num: &str) -> Result<Number, ParserError> {
    if num.contains('.') {
        let number: f64 = num
            .parse()
            .map_err(|e| ParserError::InvalidTokenFormat(format!("{e}")))?;
        Ok(Number::FloatLiteral(number))
    } else {
        let number: i64 = num
            .parse()
            .map_err(|e| ParserError::InvalidTokenFormat(format!("{e}")))?;
        Ok(Number::IntegerLiteral(number))
    }
}
