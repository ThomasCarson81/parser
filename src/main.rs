use std::env;
use std::iter::Peekable;
use std::slice::Iter;
use syntax::{Token, TokenType};

pub struct Program {
    pub statements: Vec<Statement>,
}
#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        identifier: String,
        expression: Expression,
    },
    Assignment {
        identfier: String,
        expression: Expression,
    },
    If {
        condition: Expression,
        body: Vec<Statement>,
    },

    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        initialiser: Option<Box<Statement>>,
        condition: Option<Expression>,
        increment: Option<Box<Statement>>,
        body: Vec<Statement>,
    },
    ExpressionStatement {
        expression: Expression,
    },
    Pass,
}
#[derive(Debug, Clone)]
pub enum Expression {
    Number(f64),
    Identifier(String),
    BinaryOp {
        left: Box<Expression>,
        operator: TokenType,
        right: Box<Expression>,
    },
}

#[derive(Debug)]
enum ParserError {
    UnexpectedToken(Token, TokenType),
    UnknownTokenType(String),
    InvalidTokenFormat(String),
    ExpressionError,
    CouldNotReadFile(String),
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        println!(
            "Please provide a file to parse, and optionally an output file to write the AST to."
        );
        return;
    }

    let tokens = get_tokens_from_file(&args[1]).unwrap_or_else(|e| {
        println!("Error reading tokens from file: {:?}", e);
        std::process::exit(1);
    });

    let program = match parse_program(&tokens) {
        Ok(program) => program,
        Err(e) => {
            match e {
                ParserError::UnexpectedToken(got, expected) => {
                    println!(
                        "Unexpected token: {} at line {}, Expected: {}, got '{}'",
                        got.token_type, got.line, expected, got.lexeme
                    );
                }
                ParserError::UnknownTokenType(token_type) => {
                    println!("Unknown token type: {}", token_type);
                }
                ParserError::InvalidTokenFormat(msg) => {
                    println!("Invalid token format: {}", msg);
                }
                ParserError::CouldNotReadFile(msg) => {
                    println!("Could not read file: {}", msg);
                }
                ParserError::ExpressionError => {
                    println!("Error parsing expression");
                }
            }
            return;
        }
    };
    if args.len() == 3 {
        // output result to file
    } else {
        for statement in program.statements {
            println!("{:?}", statement);
        }
    }
}

fn get_tokens_from_file(file_path: &str) -> Result<Vec<Token>, ParserError> {
    let mut tokens: Vec<Token> = Vec::new();
    let file_content = std::fs::read_to_string(&file_path)
        .map_err(|e| ParserError::CouldNotReadFile(format!("Could not read file: {}", e)))?;
    for line in file_content.lines().into_iter() {
        let token_data: Vec<&str> = line.split(',').collect();
        let token_type = TokenType::from_str(token_data[1])
            .ok_or_else(|| ParserError::UnknownTokenType(token_data[1].into()))?;
        let line_number = token_data[0].parse::<usize>().map_err(|_| {
            ParserError::InvalidTokenFormat(format!("Invalid line number in token: {}", line))
        })?;
        tokens.push(Token {
            token_type: token_type,
            lexeme: token_data[2].into(),
            line: line_number,
        });
    }
    Ok(tokens)
}

fn consume_token(
    tokens: &mut Peekable<Iter<Token>>,
    expected: TokenType,
) -> Result<Token, ParserError> {
    let peeked_token = tokens.peek().unwrap();
    let peeked_data = Token {
        token_type: peeked_token.token_type,
        lexeme: peeked_token.lexeme.clone(),
        line: peeked_token.line,
    };
    if peeked_token.token_type != expected {
        return Err(ParserError::UnexpectedToken(peeked_data, expected));
    }
    tokens.next();
    Ok(peeked_data)
}

fn check_token(tokens: &mut Peekable<Iter<Token>>, expected: TokenType) -> bool {
    let peeked_token = tokens.peek().unwrap();
    peeked_token.token_type == expected
}

fn parse_program(tokens: &[Token]) -> Result<Program, ParserError> {
    let mut statements: Vec<Statement> = Vec::new();
    let mut token_stream = tokens.iter().peekable();

    while token_stream.peek().unwrap().token_type != TokenType::EndOfFile {
        let statement = parse_statement(&mut token_stream, true)?;
        statements.push(statement);
    }
    Ok(Program { statements })
}

fn parse_statement(
    tokens: &mut Peekable<Iter<Token>>,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    let statement = match tokens.peek().unwrap().token_type {
        TokenType::LetTok => parse_declaration(tokens, consume_semicolon)?,
        TokenType::Identifier => parse_assignment(tokens, consume_semicolon)?,
        TokenType::IfTok => parse_if_statement(tokens)?,
        TokenType::WhileTok => parse_while_statement(tokens)?,
        TokenType::ForTok => parse_for_statement(tokens)?,
        _ => Statement::ExpressionStatement {
            expression: parse_expression(tokens)?,
        },
    };
    Ok(statement)
}

fn parse_declaration(
    tokens: &mut Peekable<Iter<Token>>,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::LetTok)?;
    let ident = consume_token(tokens, TokenType::Identifier)?;
    consume_token(tokens, TokenType::Equals)?;
    let expr = parse_expression(tokens)?;
    if consume_semicolon {
        consume_token(tokens, TokenType::Semicolon)?;
    }
    Ok(Statement::Let {
        identifier: ident.lexeme,
        expression: expr,
    })
}

fn parse_assignment(
    tokens: &mut Peekable<Iter<Token>>,
    consume_semicolon: bool,
) -> Result<Statement, ParserError> {
    let ident = consume_token(tokens, TokenType::Identifier)?;
    consume_token(tokens, TokenType::Equals)?;
    let expr = parse_expression(tokens)?;
    if consume_semicolon {
        consume_token(tokens, TokenType::Semicolon)?;
    }
    Ok(Statement::Let {
        identifier: ident.lexeme,
        expression: expr,
    })
}

fn parse_if_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::IfTok)?;
    let condition = parse_expression(tokens)?;
    let body: Vec<Statement> = parse_block(tokens)?;
    Ok(Statement::If {
        condition: condition,
        body: body,
    })
}

fn parse_while_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::WhileTok)?;
    let condition = parse_expression(tokens)?;
    let body = parse_block(tokens)?;
    Ok(Statement::While {
        condition: condition,
        body: body,
    })
}

fn parse_for_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    consume_token(tokens, TokenType::ForTok)?;
    consume_token(tokens, TokenType::LeftBracket)?;
    let init = if check_token(tokens, TokenType::LetTok) {
        Some(Box::new(parse_declaration(tokens, false)?))
    } else {
        None
    };
    consume_token(tokens, TokenType::Semicolon)?;
    let condition = if !check_token(tokens, TokenType::Semicolon) {
        Some(parse_expression(tokens)?)
    } else {
        None
    };
    consume_token(tokens, TokenType::Semicolon)?;
    let increment = if !check_token(tokens, TokenType::RightBracket) {
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

fn parse_expression(tokens: &mut Peekable<Iter<Token>>) -> Result<Expression, ParserError> {
    let mut expr = parse_term(tokens)?;

    while check_token(tokens, TokenType::Plus) || check_token(tokens, TokenType::Minus) {
        let operator = tokens.next().unwrap().clone();
        let right = parse_term(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
        };
    }

    Ok(expr)
}

fn parse_term(tokens: &mut Peekable<Iter<Token>>) -> Result<Expression, ParserError> {
    let mut expr = parse_primary(tokens)?;

    while check_token(tokens, TokenType::Asterisk) || check_token(tokens, TokenType::Slash) {
        let operator = tokens.next().unwrap().clone();
        let right = parse_primary(tokens)?;

        expr = Expression::BinaryOp {
            left: Box::new(expr),
            operator: operator.token_type,
            right: Box::new(right),
        };
    }

    Ok(expr)
}

fn parse_primary(tokens: &mut Peekable<Iter<Token>>) -> Result<Expression, ParserError> {
    let next_token = tokens.peek().unwrap().token_type;
    match next_token {
        TokenType::Number => {
            let num = consume_token(tokens, TokenType::Number)?;
            Ok(Expression::Number(str::parse(&num.lexeme).unwrap()))
        }
        TokenType::Identifier => {
            let ident = consume_token(tokens, TokenType::Identifier)?;

            Ok(Expression::Identifier(ident.lexeme))
        }
        _ => Err(ParserError::ExpressionError),
    }
}

fn parse_block(tokens: &mut Peekable<Iter<Token>>) -> Result<Vec<Statement>, ParserError> {
    consume_token(tokens, TokenType::LeftSquiggly)?;

    let mut statements = Vec::new();

    while !check_token(tokens, TokenType::RightSquiggly) {
        statements.push(parse_statement(tokens, true)?);
    }

    consume_token(tokens, TokenType::RightSquiggly)?;

    Ok(statements)
}
