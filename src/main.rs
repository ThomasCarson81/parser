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
    If {
        condition: Expression,
        body: Vec<Statement>,
    },

    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        initializer: Box<Statement>,
        condition: Expression,
        increment: Box<Statement>,
        body: Vec<Statement>,
    },
    ExpressionStatement {
        expression: Expression,
    },
    Pass,
}
#[derive(Debug, Clone)]
pub enum Expression {
    Number(i64),
    Identifier(String),
    BinaryOp {
        left: Box<Expression>,
        operator: TokenType,
        right: Box<Expression>,
    },
}

#[derive(Debug)]
enum ParserError {
    UnexpectedToken(Token),
    UnexpectedEOF,
    UnknownTokenType(String),
    InvalidTokenFormat(String),
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
                ParserError::UnexpectedToken(token) => {
                    println!(
                        "Unexpected token: {} at line {}",
                        token.token_type, token.line
                    );
                }
                ParserError::UnexpectedEOF => {
                    println!("Unexpected end of file");
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
            }
            return;
        }
    };
    if args.len() == 3 {
        // output result to file
    } else {
        // print result to console
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
        return Err(ParserError::UnexpectedToken(peeked_data));
    }
    Ok(peeked_data)
}

fn parse_program(tokens: &[Token]) -> Result<Program, ParserError> {
    let mut statements: Vec<Statement> = Vec::new();
    let mut token_stream = tokens.iter().peekable();

    while token_stream.peek().unwrap().token_type != TokenType::EndOfFile {
        let statement = parse_statement(&mut token_stream)?;
        statements.push(statement);
        token_stream.next();
    }
    Ok(Program { statements })
}

fn parse_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    let statement = match tokens.peek().unwrap().token_type {
        TokenType::LetTok => parse_declaration(tokens)?,
        TokenType::IfTok => parse_if_statement(tokens)?,
        TokenType::WhileTok => parse_while_statement(tokens)?,
        TokenType::ForTok => parse_for_statement(tokens)?,

        _ => Statement::ExpressionStatement {
            expression: parse_expression(tokens)?,
        },
    };
    Ok(statement)
}

fn parse_declaration(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    Ok(Statement::Pass)
}

fn parse_if_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    Ok(Statement::Pass)
}

fn parse_while_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    Ok(Statement::Pass)
}

fn parse_for_statement(tokens: &mut Peekable<Iter<Token>>) -> Result<Statement, ParserError> {
    Ok(Statement::Pass)
}

fn parse_expression(tokens: &mut Peekable<Iter<Token>>) -> Result<Expression, ParserError> {
    // Implement the parsing logic for expressions here
    Ok(Expression::Number(0))
}
