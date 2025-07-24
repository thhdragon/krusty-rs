//! Advanced, extensible, zero-copy G-code parser prototype
//! Inspired by gcode, async-gcode, and tree-sitter-gcode
//! Provides span tracking, trait-based extensibility, and robust error handling

use std::ops::Range;
use async_trait::async_trait;

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
#[async_trait]
pub trait MacroExpander: Send + Sync {
    async fn expand(&self, name: &str, args: &str) -> Option<Vec<OwnedGCodeCommand>>;
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
    macro_expander: Option<&'a (dyn MacroExpander + Send + Sync)>,
    macro_buffer: Option<std::vec::IntoIter<OwnedGCodeCommand>>, // Buffer for expanded macro commands
}

impl<'a> GCodeParser<'a> {
    pub fn new(src: &'a str, config: GCodeParserConfig) -> Self {
        Self { src, pos: 0, config, macro_expander: None, macro_buffer: None }
    }
    pub fn with_macro_expander(mut self, expander: &'a (dyn MacroExpander + Send + Sync)) -> Self {
        self.macro_expander = Some(expander);
        self
    }
    /// Iterator interface: parses next command, comment, or macro (with expansion)
    pub fn next_command(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        // If we have expanded macro commands buffered, yield those first
        if let Some(ref mut buf) = self.macro_buffer {
            if let Some(cmd) = buf.next() {
                // Convert OwnedGCodeCommand back to GCodeCommand if possible
                // For simplicity, just return an error if not possible
                // In practice, you may want to keep everything as OwnedGCodeCommand
                return Some(Ok(match cmd {
                    OwnedGCodeCommand::Word { letter, value, span } => GCodeCommand::Word { letter, value: Box::leak(value.into_boxed_str()), span },
                    OwnedGCodeCommand::Comment(comment, span) => GCodeCommand::Comment(Box::leak(comment.into_boxed_str()), span),
                    OwnedGCodeCommand::Macro { name, args, span } => GCodeCommand::Macro { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::VendorExtension { name, args, span } => GCodeCommand::VendorExtension { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::Checksum { command, checksum, span } => GCodeCommand::Checksum { command: Box::new(GCodeCommand::Word { letter: 'N', value: "0", span: span.clone() }), checksum, span },
                }));
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
                break;
            }
            let start = self.pos;
            let c = bytes[self.pos] as char;
            // Comment parsing (semicolon to end of line)
            if self.config.enable_comments && c == ';' {
                let comment_start = self.pos + 1;
                while self.pos < len && bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                let comment = &self.src[comment_start..self.pos].trim();
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::Comment(comment, span)));
            }
            // Macro parsing (e.g., {macro_name args})
            if self.config.enable_macros && c == '{' {
                let macro_start = self.pos + 1;
                let mut macro_end = macro_start;
                while macro_end < len && bytes[macro_end] != b'}' {
                    macro_end += 1;
                }
                if macro_end >= len {
                    let span = GCodeSpan { range: start..len };
                    self.pos = len;
                    return Some(Err(GCodeError { message: "Unclosed macro".to_string(), span }));
                }
                let macro_body = &self.src[macro_start..macro_end];
                let mut parts = macro_body.splitn(2, ' ');
                let name = parts.next().unwrap_or("");
                let args = parts.next().unwrap_or("");
                let span = GCodeSpan { range: start..macro_end + 1 };
                self.pos = macro_end + 1;
                // Macro expansion
                if let Some(expander) = self.macro_expander {
                    // Synchronous expansion is not supported; just return the macro
                    return Some(Ok(GCodeCommand::Macro { name, args, span }));
                } else {
                    return Some(Ok(GCodeCommand::Macro { name, args, span }));
                }
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
                // For simplicity, treat as a word
                let letter = c;
                self.pos += 1;
                let value_start = self.pos;
                while self.pos < len && (bytes[self.pos].is_ascii_alphanumeric() || bytes[self.pos] == b'.' || bytes[self.pos] == b'-' || bytes[self.pos] == b'+') {
                    self.pos += 1;
                }
                let value = &self.src[value_start..self.pos];
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::VendorExtension { name: value, args: "", span }));
            }
            // Checksum parsing (e.g., N123 G1 X10*71)
            if self.config.enable_checksums && c == 'N' {
                // For simplicity, treat as a word
                let letter = c;
                self.pos += 1;
                let value_start = self.pos;
                while self.pos < len && (bytes[self.pos].is_ascii_digit()) {
                    self.pos += 1;
                }
                let value = &self.src[value_start..self.pos];
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::Word { letter, value, span }));
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
    /// Async version: parses next command, comment, or macro (with async expansion)
    pub async fn next_command_async(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        loop {
            // If we have expanded macro commands buffered, yield those first
            if let Some(ref mut buf) = self.macro_buffer {
                if let Some(cmd) = buf.next() {
                    return Some(Ok(match cmd {
                        OwnedGCodeCommand::Word { letter, value, span } => GCodeCommand::Word { letter, value: Box::leak(value.into_boxed_str()), span },
                        OwnedGCodeCommand::Comment(comment, span) => GCodeCommand::Comment(Box::leak(comment.into_boxed_str()), span),
                        OwnedGCodeCommand::Macro { name, args, span } => GCodeCommand::Macro { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                        OwnedGCodeCommand::VendorExtension { name, args, span } => GCodeCommand::VendorExtension { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                        OwnedGCodeCommand::Checksum { command, checksum, span } => GCodeCommand::Checksum { command: Box::new(GCodeCommand::Word { letter: 'N', value: "0", span: span.clone() }), checksum, span },
                    }));
                } else {
                    self.macro_buffer = None;
                }
            }
            let bytes = self.src.as_bytes();
            let len = bytes.len();
            while self.pos < len && bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos >= len {
                break;
            }
            let start = self.pos;
            let c = bytes[self.pos] as char;
            // Comment parsing (semicolon to end of line)
            if self.config.enable_comments && c == ';' {
                let comment_start = self.pos + 1;
                while self.pos < len && bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                let comment = &self.src[comment_start..self.pos].trim();
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::Comment(comment, span)));
            }
            // Macro parsing (e.g., {macro_name args})
            if self.config.enable_macros && c == '{' {
                let macro_start = self.pos + 1;
                let mut macro_end = macro_start;
                while macro_end < len && bytes[macro_end] != b'}' {
                    macro_end += 1;
                }
                if macro_end >= len {
                    let span = GCodeSpan { range: start..len };
                    self.pos = len;
                    return Some(Err(GCodeError { message: "Unclosed macro".to_string(), span }));
                }
                let macro_body = &self.src[macro_start..macro_end];
                let mut parts = macro_body.splitn(2, ' ');
                let name = parts.next().unwrap_or("");
                let args = parts.next().unwrap_or("");
                let span = GCodeSpan { range: start..macro_end + 1 };
                self.pos = macro_end + 1;
                // Macro expansion
                if let Some(expander) = self.macro_expander {
                    if let Some(expanded) = expander.expand(name, args).await {
                        self.macro_buffer = Some(expanded.into_iter());
                        continue;
                    } else {
                        return Some(Ok(GCodeCommand::Macro { name, args, span }));
                    }
                } else {
                    return Some(Ok(GCodeCommand::Macro { name, args, span }));
                }
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
                // For simplicity, treat as a word
                let letter = c;
                self.pos += 1;
                let value_start = self.pos;
                while self.pos < len && (bytes[self.pos].is_ascii_alphanumeric() || bytes[self.pos] == b'.' || bytes[self.pos] == b'-' || bytes[self.pos] == b'+') {
                    self.pos += 1;
                }
                let value = &self.src[value_start..self.pos];
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::VendorExtension { name: value, args: "", span }));
            }
            // Checksum parsing (e.g., N123 G1 X10*71)
            if self.config.enable_checksums && c == 'N' {
                // For simplicity, treat as a word
                let letter = c;
                self.pos += 1;
                let value_start = self.pos;
                while self.pos < len && (bytes[self.pos].is_ascii_digit()) {
                    self.pos += 1;
                }
                let value = &self.src[value_start..self.pos];
                let span = GCodeSpan { range: start..self.pos };
                return Some(Ok(GCodeCommand::Word { letter, value, span }));
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
    macro_expander: Option<Box<dyn MacroExpander + Send + Sync>>,
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
    pub fn with_macro_expander(mut self, expander: Box<dyn MacroExpander + Send + Sync>) -> Self {
        self.macro_expander = Some(expander);
        self
    }
}

impl<R: AsyncBufRead + Unpin> Stream for AsyncGCodeParser<R> {
    type Item = Result<OwnedGCodeCommand, OwnedGCodeError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        // If we have commands queued from previous lines, yield them first
        if let Some(cmd) = this.command_queue.pop_front() {
            return Poll::Ready(Some(cmd));
        }
        if this.done {
            return Poll::Ready(None);
        }
        // Read the next line asynchronously
        let mut buf = std::mem::take(&mut this.buffer);
        let mut reader_pin = Pin::new(&mut this.reader);
        let mut to_consume = 0;
        let mut found_newline = false;
        match futures_util::ready!(AsyncBufRead::poll_fill_buf(reader_pin.as_mut(), cx)) {
            Ok(data) if data.is_empty() => {
                this.done = true;
                return Poll::Ready(None);
            }
            Ok(data) => {
                if let Some(pos) = data.iter().position(|&b| b == b'\n') {
                    let line = &data[..=pos];
                    buf.push_str(&String::from_utf8_lossy(line));
                    to_consume = pos + 1;
                    found_newline = true;
                } else {
                    buf.push_str(&String::from_utf8_lossy(data));
                    to_consume = data.len();
                }
            }
            Err(e) => {
                this.done = true;
                return Poll::Ready(Some(Err(OwnedGCodeError {
                    message: format!("I/O error: {}", e),
                    span: GCodeSpan { range: 0..0 },
                })));
            }
        }
        // Drop the borrow of data before calling consume
        AsyncBufRead::consume(reader_pin.as_mut(), to_consume);
        if !found_newline {
            this.buffer = buf;
            return Poll::Pending;
        }
        // Parse the buffered line
        let line = buf.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            this.buffer.clear();
            return self.poll_next(cx);
        }
        let mut parser = GCodeParser::new(line, this.config.clone());
        if let Some(expander) = this.macro_expander.as_deref() {
            parser = parser.with_macro_expander(expander);
        }
        while let Some(cmd) = futures::executor::block_on(async { parser.next_command_async().await }) {
            match cmd {
                Ok(cmd) => {
                    this.command_queue.push_back(Ok(OwnedGCodeCommand::from(cmd)));
                }
                Err(e) => {
                    this.command_queue.push_back(Err(OwnedGCodeError::from(e)));
                    // On error, skip to next line (do not parse further in this line)
                    break;
                }
            }
        }
        this.buffer.clear();
        // Yield the next command or error
        if let Some(cmd) = this.command_queue.pop_front() {
            Poll::Ready(Some(cmd))
        } else {
            // If nothing was parsed, try the next line
            self.poll_next(cx)
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
                _ => f64::NAN,
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
                    _ => f64::NAN,
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
            if let Ok(num) = tok.parse::<f64>() {
                (Expr::Number(num), rest)
            } else if tok == "-" {
                let (rhs, rest) = parse_expr_bp(rest, 100)?;
                (Expr::UnaryOp { op: '-', rhs: Box::new(rhs) }, rest)
            } else if tok == "+" {
                let (rhs, rest) = parse_expr_bp(rest, 100)?;
                (Expr::UnaryOp { op: '+', rhs: Box::new(rhs) }, rest)
            } else if tok == "(" {
                let (expr, rest) = parse_expr_bp(rest, 0)?;
                if let Some((close, rest)) = rest.split_first() {
                    if close == ")" {
                        (expr, rest)
                    } else {
                        return Err("Expected ')'".to_string());
                    }
                } else {
                    return Err("Unclosed parenthesis".to_string());
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
            "^" => (5, 6),
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

// Usage example (to be expanded in docs/tests)
// let mut parser = GCodeParser::new(gcode_str, GCodeParserConfig::default());
// while let Some(cmd) = parser.next_command() {
//     match cmd {
//         Ok(cmd) => { /* handle command */ },
//         Err(err) => { /* handle error */ },
//     }
// }
