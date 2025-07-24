use async_stream::stream;
pub struct AsyncGCodeParser<R: AsyncBufRead + Unpin + Send + 'static> {
    reader: R,
    config: GCodeParserConfig,
    macro_expander: Option<Box<dyn MacroExpander + Send + Sync>>,
}

impl<R: AsyncBufRead + Unpin + Send + 'static> AsyncGCodeParser<R> {
    pub fn new(reader: R, config: GCodeParserConfig) -> Self {
        Self {
            reader,
            config,
            macro_expander: None,
        }
    }
    pub fn with_macro_expander(mut self, expander: Box<dyn MacroExpander + Send + Sync>) -> Self {
        self.macro_expander = Some(expander);
        self
    }

    pub fn into_stream(self) -> impl Stream<Item = Result<OwnedGCodeCommand, OwnedGCodeError>> + Send {
        let mut reader = self.reader;
        let config = self.config;
        let macro_expander = self.macro_expander;
        stream! {
            use futures_util::AsyncBufReadExt;
            let mut buf = String::new();
            loop {
                buf.clear();
                let bytes = reader.read_line(&mut buf).await.unwrap_or(0);
                if bytes == 0 {
                    break;
                }
                let mut parser = GCodeParser::new(&buf, config.clone());
                while let Some(cmd_result) = parser.next_command() {
                    match cmd_result {
                        Ok(GCodeCommand::Macro { name, args, span }) => {
                            if let Some(expander) = &macro_expander {
                                let expanded = expander.expand(name.to_string(), args.to_string()).await;
                                if let Some(commands) = expanded {
                                    for cmd in commands {
                                        yield Ok(cmd);
                                    }
                                } else {
                                    yield Err(OwnedGCodeError {
                                        message: format!("Macro '{}' not found", name),
                                        span: span.clone(),
                                    });
                                }
                            }
                        }
                        Ok(cmd) => yield Ok(cmd.into()),
                        Err(e) => yield Err(e.into()),
                    }
                }
            }
        }
    }
}
// --- Real GCodeParser Implementation ---
pub struct GCodeParser<'a> {
    input: &'a str,
    config: GCodeParserConfig,
    pos: usize,
}

impl<'a> GCodeParser<'a> {
    pub fn new(input: &'a str, config: GCodeParserConfig) -> Self {
        Self { input, config, pos: 0 }
    }

    fn parse_word(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        let start = self.pos;
        let c = self.input[self.pos..].chars().next().unwrap();
        self.pos += 1;
        let value_start = self.pos;
        while self.pos < self.input.len() && !self.input[self.pos..].starts_with('\n') {
            self.pos += 1;
        }
        let value = &self.input[value_start..self.pos].trim();
        Some(Ok(GCodeCommand::Word {
            letter: c,
            value,
            span: GCodeSpan { range: start..self.pos },
        }))
    }

    pub fn next_command(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        while self.pos < self.input.len() && self.input[self.pos..].starts_with(char::is_whitespace) {
            self.pos += 1;
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let c = self.input[self.pos..].chars().next().unwrap();
        match c {
            ';' if self.config.enable_comments => {
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos..].chars().next().unwrap() != '\n' {
                    self.pos += 1;
                }
                Some(Ok(GCodeCommand::Comment(&self.input[start..self.pos], GCodeSpan { range: start..self.pos })))
            }
            '{' if self.config.enable_macros => {
                let start = self.pos;
                self.pos += 1;
                let name_start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos..].chars().next().unwrap() != '}' {
                    self.pos += 1;
                }
                let name = &self.input[name_start..self.pos];
                self.pos += 1;
                Some(Ok(GCodeCommand::Macro {
                    name,
                    args: "",
                    span: GCodeSpan { range: start..self.pos },
                }))
            }
            _ if c.is_alphabetic() => self.parse_word(),
            _ => {
                let start = self.pos;
                self.pos = self.input.len();
                Some(Err(GCodeError {
                    message: format!("Unexpected character: {}", c),
                    span: GCodeSpan { range: start..self.pos },
                }))
            }
        }
    }
}
/// G-code parser: sync and async, with macro and infix support
use std::ops::Range;

// --- Core Types ---

#[derive(Debug, Clone, PartialEq)]
pub struct GCodeSpan {
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GCodeCommand<'a> {
    Word { letter: char, value: &'a str, span: GCodeSpan },
    Comment(&'a str, GCodeSpan),
    Macro { name: &'a str, args: &'a str, span: GCodeSpan },
    VendorExtension { name: &'a str, args: &'a str, span: GCodeSpan },
    Checksum { command: Box<GCodeCommand<'a>>, checksum: u8, span: GCodeSpan },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GCodeError {
    pub message: String,
    pub span: GCodeSpan,
}

impl std::fmt::Display for GCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {:?})", self.message, self.span)
    }
}

impl std::error::Error for GCodeError {}

#[derive(Debug, Clone, PartialEq)]
pub struct GCodeParserConfig {
    pub enable_comments: bool,
    pub enable_checksums: bool,
    pub enable_infix: bool,
    pub enable_macros: bool,
    pub enable_vendor_extensions: bool,
}

impl Default for GCodeParserConfig {
    fn default() -> Self {
        Self {
            enable_comments: true,
            enable_checksums: true,
            enable_infix: true,
            enable_macros: true,
            enable_vendor_extensions: true,
        }
    }
}

// --- Owned Types for Async/Streaming ---

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedGCodeCommand {
    Word { letter: char, value: String, span: GCodeSpan },
    Comment(String, GCodeSpan),
    Macro { name: String, args: String, span: GCodeSpan },
    VendorExtension { name: String, args: String, span: GCodeSpan },
    Checksum { command: Box<OwnedGCodeCommand>, checksum: u8, span: GCodeSpan },
}

impl<'a> From<GCodeCommand<'a>> for OwnedGCodeCommand {
    fn from(cmd: GCodeCommand<'a>) -> Self {
        match cmd {
            GCodeCommand::Word { letter, value, span } => Self::Word { letter, value: value.to_string(), span },
            GCodeCommand::Comment(comment, span) => Self::Comment(comment.to_string(), span),
            GCodeCommand::Macro { name, args, span } => Self::Macro { name: name.to_string(), args: args.to_string(), span },
            GCodeCommand::VendorExtension { name, args, span } => Self::VendorExtension { name: name.to_string(), args: args.to_string(), span },
            GCodeCommand::Checksum { command, checksum, span } => Self::Checksum { command: Box::new(OwnedGCodeCommand::from(*command)), checksum, span },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedGCodeError {
    pub message: String,
    pub span: GCodeSpan,
}

impl From<GCodeError> for OwnedGCodeError {
    fn from(e: GCodeError) -> Self {
        OwnedGCodeError { message: e.message, span: e.span }
    }
}

// --- MacroExpander Trait ---

#[async_trait::async_trait]
pub trait MacroExpander: Send + Sync {
    async fn expand(&self, name: String, args: String) -> Option<Vec<OwnedGCodeCommand>>;
}

// --- Async Streaming G-code Parser ---

use futures_util::io::AsyncBufRead;
// use futures_util::AsyncBufReadExt; // No longer needed
use futures_core::stream::Stream;

// New struct will be added below

// use std::future::Future; // Already in scope via std::future::Future in trait bounds

// Removed: AsyncParserState, replaced by async-stream generator

// Removed duplicate impl block for AsyncGCodeParser<R>

// Old Stream implementation removed; use into_stream() for async streaming.

// --- Infix Expression Parsing (Pratt Parser) ---

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    UnaryOp { op: char, rhs: Box<Expr> },
    BinaryOp { lhs: Box<Expr>, op: char, rhs: Box<Expr> },
}

impl Expr {
    pub fn eval(&self) -> f64 {
        match self {
            Expr::Number(n) => *n,
            Expr::UnaryOp { op, rhs } => match op {
                '+' => rhs.eval(),
                '-' => -rhs.eval(),
                _ => panic!("Unknown unary op: {}", op),
            },
            Expr::BinaryOp { lhs, op, rhs } => {
                let l = lhs.eval();
                let r = rhs.eval();
                match op {
                    '+' => l + r,
                    '-' => l - r,
                    '*' => l * r,
                    '/' => l / r,
                    '^' => l.powf(r),
                    _ => panic!("Unknown binary op: {}", op),
                }
            }
        }
    }
}

/// Parse an infix expression from a &str (returns Expr or error string)
pub fn parse_infix_expr(input: &str) -> Result<Expr, String> {
    let tokens = tokenize_expr(input)?;
    let (expr, rest) = parse_expr_bp(&tokens, 0)?;
    if !rest.is_empty() {
        return Err("Unexpected tokens after expression".to_string());
    }
    Ok(expr)
}

fn tokenize_expr(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c.is_ascii_digit() || c == '.' {
            let mut num = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() || d == '.' {
                    num.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(num);
        } else if "+-*/^()".contains(c) {
            tokens.push(c.to_string());
            chars.next();
        } else {
            return Err(format!("Unexpected character in expression: {}", c));
        }
    }
    Ok(tokens)
}

// Pratt parser with binding power
fn parse_expr_bp(tokens: &[String], min_bp: u8) -> Result<(Expr, &[String]), String> {
    let (mut lhs, mut rest) = match tokens.split_first() {
        Some((tok, rest)) => {
            if let Ok(n) = tok.parse::<f64>() {
                (Expr::Number(n), rest)
            } else if tok == "-" {
                let (rhs, rest) = parse_expr_bp(rest, 100)?;
                (Expr::UnaryOp { op: '-', rhs: Box::new(rhs) }, rest)
            } else if tok == "+" {
                let (rhs, rest) = parse_expr_bp(rest, 100)?;
                (Expr::UnaryOp { op: '+', rhs: Box::new(rhs) }, rest)
            } else if tok == "(" {
                let (expr, rest) = parse_expr_bp(rest, 0)?;
                if let Some((close, rest2)) = rest.split_first() {
                    if close == ")" {
                        (expr, rest2)
                    } else {
                        return Err("Expected ')'".to_string());
                    }
                } else {
                    return Err("Expected ')'".to_string());
                }
            } else {
                return Err(format!("Unexpected token: {}", tok));
            }
        }
        None => return Err("Unexpected end of input".to_string()),
    };
    loop {
        let op = match rest.first() {
            Some(op) if ["+", "-", "*", "/", "^"].contains(&op.as_str()) => op,
            _ => break,
        };
        let (l_bp, r_bp) = match op.as_str() {
            "+" | "-" => (1, 2),
            "*" | "/" => (3, 4),
            "^" => (5, 4),
            _ => break,
        };
        if l_bp < min_bp {
            break;
        }
        let op_char = op.chars().next().unwrap();
        let rest2 = &rest[1..];
        let (rhs, new_rest) = parse_expr_bp(rest2, r_bp)?;
        lhs = Expr::BinaryOp { lhs: Box::new(lhs), op: op_char, rhs: Box::new(rhs) };
        rest = new_rest;
    }
    Ok((lhs, rest))
}
