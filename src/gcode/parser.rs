// Synchronous GCodeParser stub (to be replaced with real implementation)
// --- Synchronous GCodeParser stub (to be replaced with real implementation) ---
// This is needed for async parser integration and macro expansion.
pub struct GCodeParser<'a> {
    input: &'a str,
    config: GCodeParserConfig,
    pos: usize,
}

impl<'a> GCodeParser<'a> {
    pub fn new(input: &'a str, config: GCodeParserConfig) -> Self {
        Self { input, config, pos: 0 }
    }
    pub fn next_command(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        // Dummy implementation: treat each line as a comment for now
        if self.pos >= self.input.len() {
            return None;
        }
        let end = self.input[self.pos..].find('\n').map(|i| self.pos + i + 1).unwrap_or(self.input.len());
        let line = &self.input[self.pos..end];
        self.pos = end;
        if line.trim().is_empty() {
            return self.next_command();
        }
        Some(Ok(GCodeCommand::Comment(line.trim(), GCodeSpan { range: 0..line.len() })))
    }
}
/// G-code parser: sync and async, with macro and infix support
use std::ops::Range;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

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
    async fn expand(&self, name: &str, args: &str) -> Option<Vec<OwnedGCodeCommand>>;
}

// --- Async Streaming G-code Parser ---

use futures_util::io::AsyncBufRead;
use futures_core::stream::Stream;

pub struct AsyncGCodeParser<R: AsyncBufRead + Unpin> {
    reader: R,
    buffer: String,
    command_queue: VecDeque<Result<OwnedGCodeCommand, OwnedGCodeError>>,
    config: GCodeParserConfig,
    macro_expander: Option<Box<dyn MacroExpander + Send + Sync>>,
    done: bool,
    state: AsyncParserState<R>,
}

pub enum AsyncParserState<R: AsyncBufRead + Unpin> {
    /// Reading a line from the input stream
    ReadingLine,
    /// Expanding a macro asynchronously (future in progress)
    ExpandingMacro {
        fut: Pin<Box<dyn Future<Output = Option<Vec<OwnedGCodeCommand>>> + Send>>,
        macro_name: String,
        macro_args: String,
        macro_span: GCodeSpan,
    },
    /// Yielding a command from the command queue
    YieldingCommand,
    /// Parsing is complete (EOF or error)
    Done,
    /// Phantom state for type completeness
    _Phantom(std::marker::PhantomData<R>),
}

impl<R: AsyncBufRead + Unpin> AsyncGCodeParser<R> {
    pub fn new(reader: R, config: GCodeParserConfig) -> Self {
        Self {
            reader,
            buffer: String::new(),
            command_queue: VecDeque::new(),
            config,
            macro_expander: None,
            done: false,
            state: AsyncParserState::ReadingLine,
        }
    }
    pub fn with_macro_expander(mut self, expander: Box<dyn MacroExpander + Send + Sync>) -> Self {
        self.macro_expander = Some(expander);
        self
    }
}

impl<R: AsyncBufRead + Unpin> Stream for AsyncGCodeParser<R> {
    type Item = Result<OwnedGCodeCommand, OwnedGCodeError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        // State machine for async G-code parsing
        loop {
            match &mut this.state {
                // State: Reading a line from the input
                AsyncParserState::ReadingLine => {
                    // use futures_util::AsyncBufReadExt; // No longer needed
                    if this.done {
                        this.state = AsyncParserState::Done;
                        continue;
                    }
                    // Avoid multiple mutable borrows: scope reader_pin and buffer usage
                    let mut line = String::new();
                    let bytes_to_consume = {
                        let mut reader_pin = Pin::new(&mut this.reader);
                        match futures_util::ready!(reader_pin.as_mut().poll_fill_buf(cx)) {
                            Ok(bytes) if bytes.is_empty() => {
                                this.done = true;
                                this.state = AsyncParserState::Done;
                                continue;
                            }
                            Ok(bytes) => {
                                let s = match std::str::from_utf8(bytes) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        let err = OwnedGCodeError {
                                            message: format!("Invalid UTF-8 in input: {}", e),
                                            span: GCodeSpan { range: 0..bytes.len() },
                                        };
                                        this.done = true;
                                        this.state = AsyncParserState::Done;
                                        return Poll::Ready(Some(Err(err)));
                                    }
                                };
                                if let Some(pos) = s.find('\n') {
                                    line.push_str(&s[..=pos]);
                                    pos + 1
                                } else {
                                    line.push_str(s);
                                    bytes.len()
                                }
                            }
                            Err(e) => {
                                let err = OwnedGCodeError {
                                    message: format!("IO error while reading: {}", e),
                                    span: GCodeSpan { range: 0..0 },
                                };
                                this.done = true;
                                this.state = AsyncParserState::Done;
                                return Poll::Ready(Some(Err(err)));
                            }
                        }
                    };
                    // Now, after the borrow is dropped, consume the bytes
                    if bytes_to_consume > 0 {
                        let reader_pin = Pin::new(&mut this.reader);
                        reader_pin.consume(bytes_to_consume);
                    }
                    this.buffer.clear();
                    this.buffer.push_str(&line);
                    // Parse the buffer into commands using the sync parser logic
                    if !this.buffer.trim().is_empty() {
                        let config = this.config.clone();
                        let mut parser = GCodeParser::new(&this.buffer, config);
                        while let Some(cmd_result) = parser.next_command() {
                            match cmd_result {
                                Ok(cmd) => {
                                    this.command_queue.push_back(Ok(OwnedGCodeCommand::from(cmd)));
                                }
                                Err(e) => {
                                    this.command_queue.push_back(Err(OwnedGCodeError::from(e)));
                                }
                            }
                        }
                    }
                    this.buffer.clear();
                    if let Some(cmd) = this.command_queue.pop_front() {
                        return Poll::Ready(Some(cmd));
                    }
                }
                // State: Expanding a macro asynchronously
                AsyncParserState::ExpandingMacro { fut, macro_name: _, macro_args: _, macro_span: _ } => {
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Some(commands)) => {
                            for cmd in commands {
                                this.command_queue.push_back(Ok(cmd));
                            }
                            this.state = AsyncParserState::YieldingCommand;
                        }
                        Poll::Ready(None) | Poll::Pending => {
                            this.state = AsyncParserState::Done;
                        }
                    }
                }
                // State: Yielding a command from the queue
                AsyncParserState::YieldingCommand => {
                    this.state = AsyncParserState::ReadingLine;
                    continue;
                }
                // State: Parsing is complete
                AsyncParserState::Done => {
                    return Poll::Ready(None);
                }
                // State: Phantom (should never occur)
                AsyncParserState::_Phantom(_) => unreachable!(),
            }
        }
    }
}

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
