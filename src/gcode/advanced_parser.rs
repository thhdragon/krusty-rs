//! Advanced, extensible, zero-copy G-code parser prototype
//! Inspired by gcode, async-gcode, and tree-sitter-gcode
//! Provides span tracking, trait-based extensibility, and robust error handling

use std::ops::Range;

/// Span in the original source text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GCodeSpan {
    pub range: Range<usize>,
}

/// G-code command or token, with span info
#[derive(Debug, Clone, PartialEq)]
pub enum GCodeCommand<'a> {
    Word { letter: char, value: &'a str, span: GCodeSpan },
    Comment(&'a str, GCodeSpan),
    Macro { name: &'a str, args: &'a str, span: GCodeSpan },
    VendorExtension { name: &'a str, args: &'a str, span: GCodeSpan },
    Checksum { command: Box<GCodeCommand<'a>>, checksum: u8, span: GCodeSpan },
    // Extend with more variants as needed
}

/// Error with span info
#[derive(Debug, Clone, PartialEq)]
pub struct GCodeError {
    pub message: String,
    pub span: GCodeSpan,
}

/// Trait for macro/custom command expansion
pub trait MacroExpander<'a> {
    fn expand(&self, name: &'a str, args: &'a str) -> Option<Vec<GCodeCommand<'a>>>;
}

/// Parser config options
#[derive(Debug, Clone, Default)]
pub struct GCodeParserConfig {
    pub enable_comments: bool,
    pub enable_checksums: bool,
    pub enable_infix: bool,
    pub enable_macros: bool,
    pub enable_vendor_extensions: bool,
}

/// Main parser struct
pub struct GCodeParser<'a> {
    src: &'a str,
    pos: usize,
    config: GCodeParserConfig,
    macro_expander: Option<&'a dyn MacroExpander<'a>>,
    macro_buffer: Option<std::vec::IntoIter<GCodeCommand<'a>>>, // Buffer for expanded macro commands
}

impl<'a> GCodeParser<'a> {
    pub fn new(src: &'a str, config: GCodeParserConfig) -> Self {
        Self { src, pos: 0, config, macro_expander: None, macro_buffer: None }
    }
    pub fn with_macro_expander(mut self, expander: &'a dyn MacroExpander<'a>) -> Self {
        self.macro_expander = Some(expander);
        self
    }
    /// Iterator interface: parses next command, comment, or macro (with expansion)
    pub fn next_command(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        // If we have expanded macro commands buffered, yield those first
        if let Some(ref mut buf) = self.macro_buffer {
            if let Some(cmd) = buf.next() {
                return Some(Ok(cmd));
            } else {
                self.macro_buffer = None;
            }
        }
        let bytes = self.src.as_bytes();
        let len = bytes.len();
        while self.pos < len {
            // Skip whitespace
            while self.pos < len && bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos >= len {
                return None;
            }
            let start = self.pos;
            let c = bytes[self.pos] as char;
            // Comment parsing (semicolon to end of line)
            if self.config.enable_comments && c == ';' {
                let comment_start = self.pos;
                let mut comment_end = self.pos;
                while comment_end < len && bytes[comment_end] != b'\n' {
                    comment_end += 1;
                }
                let comment = &self.src[comment_start + 1..comment_end];
                self.pos = comment_end;
                return Some(Ok(GCodeCommand::Comment(comment.trim(), GCodeSpan { range: comment_start..comment_end })));
            }
            // Macro parsing (e.g., {macro_name args})
            if self.config.enable_macros && c == '{' {
                let macro_start = self.pos;
                let mut macro_end = self.pos;
                while macro_end < len && bytes[macro_end] != b'}' {
                    macro_end += 1;
                }
                if macro_end >= len {
                    // Unterminated macro
                    let span = GCodeSpan { range: macro_start..len };
                    self.pos = len;
                    return Some(Err(GCodeError { message: "Unterminated macro".to_string(), span }));
                }
                let macro_body = &self.src[macro_start + 1..macro_end];
                let mut parts = macro_body.splitn(2, char::is_whitespace);
                let name = parts.next().unwrap_or("").trim();
                let args = parts.next().unwrap_or("").trim();
                self.pos = macro_end + 1;
                let span = GCodeSpan { range: macro_start..(macro_end + 1) };
                if let Some(expander) = self.macro_expander {
                    if let Some(expanded) = expander.expand(name, args) {
                        self.macro_buffer = Some(expanded.into_iter());
                        // Yield the first expanded command
                        if let Some(cmd) = self.macro_buffer.as_mut().unwrap().next() {
                            return Some(Ok(cmd));
                        } else {
                            self.macro_buffer = None;
                            continue;
                        }
                    }
                }
                return Some(Ok(GCodeCommand::Macro { name, args, span }));
            }
            // G-code word parsing (e.g., G1, X10.0)
            if c.is_ascii_alphabetic() {
                let letter = c;
                self.pos += 1;
                let value_start = self.pos;
                while self.pos < len && (bytes[self.pos].is_ascii_alphanumeric() || bytes[self.pos] == b'.' || bytes[self.pos] == b'-' || bytes[self.pos] == b'+') {
                    self.pos += 1;
                }
                let value = &self.src[value_start..self.pos];
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::Word { letter, value, span }));
            }
            // Vendor extension parsing (e.g., @command args or M900 ...)
            if self.config.enable_vendor_extensions && (c == '@' || (c == 'M' && self.pos + 1 < len && bytes[self.pos + 1].is_ascii_digit())) {
                let ext_start = self.pos;
                // Parse @command or Mxxx
                if c == '@' {
                    self.pos += 1;
                    let name_start = self.pos;
                    while self.pos < len && bytes[self.pos].is_ascii_alphabetic() {
                        self.pos += 1;
                    }
                    let name = &self.src[name_start..self.pos];
                    // Parse args (rest of line or until whitespace)
                    let args_start = self.pos;
                    while self.pos < len && !bytes[self.pos].is_ascii_whitespace() && bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                    let args = &self.src[args_start..self.pos];
                    let span = GCodeSpan { range: ext_start..self.pos };
                    return Some(Ok(GCodeCommand::VendorExtension { name, args, span }));
                } else if c == 'M' {
                    let name_start = self.pos;
                    self.pos += 1;
                    while self.pos < len && bytes[self.pos].is_ascii_digit() {
                        self.pos += 1;
                    }
                    let name = &self.src[name_start..self.pos];
                    // Parse args (rest of line)
                    let args_start = self.pos;
                    while self.pos < len && bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                    let args = &self.src[args_start..self.pos].trim();
                    let span = GCodeSpan { range: name_start..self.pos };
                    return Some(Ok(GCodeCommand::VendorExtension { name, args, span }));
                }
            }
            // Checksum parsing (e.g., N123 G1 X10*71)
            if self.config.enable_checksums && c == 'N' {
                let n_start = self.pos;
                self.pos += 1;
                while self.pos < len && bytes[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
                // Parse command after N...
                let cmd_start = self.pos;
                while self.pos < len && bytes[self.pos] != b'*' && bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                let cmd_str = &self.src[cmd_start..self.pos];
                // Parse checksum
                if self.pos < len && bytes[self.pos] == b'*' {
                    self.pos += 1;
                    let cksum_start = self.pos;
                    while self.pos < len && bytes[self.pos].is_ascii_digit() {
                        self.pos += 1;
                    }
                    let cksum_str = &self.src[cksum_start..self.pos];
                    let checksum = cksum_str.parse::<u8>().unwrap_or(0);
                    let span = GCodeSpan { range: n_start..self.pos };
                    // Parse the command inside (recursive, but only one level)
                    let mut inner_parser = GCodeParser::new(cmd_str, self.config.clone());
                    let cmd = inner_parser.next_command().unwrap_or(Err(GCodeError { message: "Invalid command in checksum".to_string(), span: span.clone() }));
                    return Some(cmd.map(|c| GCodeCommand::Checksum { command: Box::new(c), checksum, span }));
                }
            }
            // Unknown or invalid character: report error, then skip to next whitespace or line
            let err_span = GCodeSpan { range: self.pos..self.pos + 1 };
            self.pos += 1;
            // Skip to next whitespace or line to avoid infinite loop
            while self.pos < len && !bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            return Some(Err(GCodeError { message: format!("Unexpected character: {}", c), span: err_span }));
        }
        None
    }
    // TODO: Add async/streaming API, error recovery, etc.
}

use futures_core::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::io::AsyncBufRead;

/// Owned version of GCodeCommand for async/streaming use
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
            GCodeCommand::Word { letter, value, span } => OwnedGCodeCommand::Word { letter, value: value.to_string(), span },
            GCodeCommand::Comment(comment, span) => OwnedGCodeCommand::Comment(comment.to_string(), span),
            GCodeCommand::Macro { name, args, span } => OwnedGCodeCommand::Macro { name: name.to_string(), args: args.to_string(), span },
            GCodeCommand::VendorExtension { name, args, span } => OwnedGCodeCommand::VendorExtension { name: name.to_string(), args: args.to_string(), span },
            GCodeCommand::Checksum { command, checksum, span } => OwnedGCodeCommand::Checksum { command: Box::new(OwnedGCodeCommand::from(*command)), checksum, span },
        }
    }
}

/// Owned error for async/streaming use
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

/// Async streaming G-code parser for non-blocking parsing from async sources
pub struct AsyncGCodeParser<R: AsyncBufRead + Unpin> {
    reader: R,
    buffer: String,
    command_queue: std::collections::VecDeque<Result<OwnedGCodeCommand, OwnedGCodeError>>,
    config: GCodeParserConfig,
    macro_expander: Option<Box<dyn for<'b> MacroExpander<'b> + Send + Sync>>,
    done: bool,
}

impl<R: AsyncBufRead + Unpin> AsyncGCodeParser<R> {
    pub fn new(reader: R, config: GCodeParserConfig) -> Self {
        Self {
            reader,
            buffer: String::new(),
            command_queue: std::collections::VecDeque::new(),
            config,
            macro_expander: None,
            done: false,
        }
    }
    pub fn with_macro_expander(mut self, expander: Box<dyn for<'b> MacroExpander<'b> + Send + Sync>) -> Self {
        self.macro_expander = Some(expander);
        self
    }
}

impl<R: AsyncBufRead + Unpin> Stream for AsyncGCodeParser<R> {
    type Item = Result<OwnedGCodeCommand, OwnedGCodeError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(cmd) = this.command_queue.pop_front() {
            return Poll::Ready(Some(cmd));
        }
        if this.done {
            return Poll::Ready(None);
        }
        
        match Pin::new(&mut this.reader).poll_fill_buf(cx) {
            Poll::Ready(Ok(data)) if !data.is_empty() => {
                let s = std::str::from_utf8(data).unwrap_or("");
                this.buffer.push_str(s);
                let len = data.len();
                Pin::new(&mut this.reader).consume(len);
                // Drain all commands from the parser into the queue
                let src = &this.buffer;
                let mut parser = GCodeParser::new(src, this.config.clone());
                if let Some(ref expander) = this.macro_expander {
                    parser = parser.with_macro_expander(&**expander);
                }
                while let Some(cmd) = parser.next_command() {
                    this.command_queue.push_back(cmd.map(OwnedGCodeCommand::from).map_err(OwnedGCodeError::from));
                }
                if let Some(cmd) = this.command_queue.pop_front() {
                    return Poll::Ready(Some(cmd));
                }
                Poll::Pending
            }
            Poll::Ready(Ok(_)) | Poll::Ready(Err(_)) => {
                this.done = true;
                return Poll::Ready(None);
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Minimal Pratt parser for infix expressions (supports +, -, *, /, ^, parentheses)
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
                '-' => -rhs.eval(),
                '+' => rhs.eval(),
                _ => rhs.eval(),
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
                    _ => l,
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
                if let Some((close, rest)) = rest.split_first() {
                    if close != ")" {
                        return Err("Expected ')'".to_string());
                    }
                    (expr, rest)
                } else {
                    return Err("Unclosed '(' in expression".to_string());
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
            "^" => (5, 4), // right-associative
            _ => break,
        };
        if l_bp < min_bp {
            break;
        }
        let op_char = op.chars().next().unwrap();
        rest = &rest[1..];
        let (rhs, new_rest) = parse_expr_bp(rest, r_bp)?;
        lhs = Expr::BinaryOp { lhs: Box::new(lhs), op: op_char, rhs: Box::new(rhs) };
        rest = new_rest;
    }
    Ok((lhs, rest))
}

// Usage example (to be expanded in docs/tests)
// let mut parser = GCodeParser::new(gcode_str, GCodeParserConfig::default());
// while let Some(cmd) = parser.next_command() {
//     match cmd {
//         Ok(cmd) => { /* handle command */ },
//         Err(err) => { /* handle error */ },
//     }
// }
