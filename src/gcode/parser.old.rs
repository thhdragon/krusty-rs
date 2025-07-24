// --- Async streaming G-code parser for non-blocking parsing from async sources ---
use futures_util::io::AsyncBufRead;
use futures_core::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Advanced, extensible, zero-copy G-code parser prototype
/// Inspired by gcode, async-gcode, and tree-sitter-gcode

    /// Async version: parses next command, comment, or macro (with async expansion)
    pub async fn next_command_async(&mut self) -> Option<Result<GCodeCommand<'a>, GCodeError>> {
        loop {
            // If we have expanded macro commands buffered, yield those first
            if let Some(ref mut buf) = self.macro_buffer {
                if let Some(cmd) = buf.next() {
                    // Instead of leaking, return an error or use a placeholder
                    return Some(Err(GCodeError {
                        message: "Cannot convert OwnedGCodeCommand to GCodeCommand without leaking".to_string(),
                        span: match &cmd {
                            OwnedGCodeCommand::Word { span, .. }
                            | OwnedGCodeCommand::Comment(_, span)
                            | OwnedGCodeCommand::Macro { span, .. }
                            | OwnedGCodeCommand::VendorExtension { span, .. }
                            | OwnedGCodeCommand::Checksum { span, .. } => span.clone(),
                        },
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

// --- Types and Implementations moved to module scope ---

// Owned version of GCodeCommand for async/streaming use
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

// Owned error for async/streaming use
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

// Async streaming G-code parser for non-blocking parsing from async sources
use std::future::Future;
use std::pin::Pin as StdPin;
use std::collections::VecDeque;

pub struct AsyncGCodeParser<R: AsyncBufRead + Unpin> {
    reader: R,
    buffer: String,
    command_queue: VecDeque<Result<OwnedGCodeCommand, OwnedGCodeError>>,
    config: GCodeParserConfig,
    macro_expander: Option<Box<dyn MacroExpander + Send + Sync>>,
    done: bool,
    state: AsyncParserState<R>,
}


// Async parser state machine
pub enum AsyncParserState<R: AsyncBufRead + Unpin> {
    ReadingLine,
    ExpandingMacro {
        fut: StdPin<Box<dyn Future<Output = Option<Vec<OwnedGCodeCommand>>> + Send>>,
        macro_name: String,
        macro_args: String,
        macro_span: GCodeSpan,
    },
    YieldingCommand,
    Done,
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
        loop {
            // Yield any queued commands first
            if let Some(cmd) = this.command_queue.pop_front() {
                tracing::debug!("Yielding command from queue");
                return Poll::Ready(Some(cmd));
            }
            match &mut this.state {
                AsyncParserState::ReadingLine => {
                    tracing::debug!("State: ReadingLine");
                    let mut buf = std::mem::take(&mut this.buffer);
                    let mut reader_pin = StdPin::new(&mut this.reader);
                    let mut to_consume = 0;
                    let mut found_newline = false;
                    match futures_util::ready!(AsyncBufRead::poll_fill_buf(reader_pin.as_mut(), cx)) {
                        Ok(data) if data.is_empty() => {
                            this.done = true;
                            this.state = AsyncParserState::Done;
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
                            this.state = AsyncParserState::Done;
                            return Poll::Ready(Some(Err(OwnedGCodeError {
                                message: format!("I/O error: {}", e),
                                span: GCodeSpan { range: 0..0 },
                            })));
                        }
                    }
                    AsyncBufRead::consume(reader_pin.as_mut(), to_consume);
                    if !found_newline {
                        this.buffer = buf;
                        return Poll::Pending;
                    }
                    let line = buf.trim_end_matches(['\r', '\n']);
                    if line.is_empty() {
                        this.buffer.clear();
                        continue;
                    }
                    let mut parser = GCodeParser::new(line, this.config.clone());
                    if let Some(expander) = this.macro_expander.as_deref() {
                        parser = parser.with_macro_expander(expander);
                    }
                    while let Some(cmd) = parser.next_command() {
                        match cmd {
                            Ok(GCodeCommand::Macro { name, args, span }) => {
                                if let Some(expander) = this.macro_expander.as_ref() {
                                    // Instead of trying to clone the trait object, just use the reference
                                    let name = name.to_string();
                                    let args = args.to_string();
                                    let span = span.clone();
                                    let expander_ref = expander.as_ref();
                                    let fut = Box::pin(async move { expander_ref.expand(&name, &args).await });
                                    this.state = AsyncParserState::ExpandingMacro {
                                        fut,
                                        macro_name: name,
                                        macro_args: args,
                                        macro_span: span,
                                    };
                                    tracing::debug!("State: ExpandingMacro");
                                    break;
                                } else {
                                    this.command_queue.push_back(Ok(OwnedGCodeCommand::Macro {
                                        name: name.to_string(),
                                        args: args.to_string(),
                                        span: span.clone(),
                                    }));
                                }
                            }
                            Ok(cmd) => {
                                this.command_queue.push_back(Ok(OwnedGCodeCommand::from(cmd)));
                            }
                            Err(e) => {
                                this.command_queue.push_back(Err(OwnedGCodeError::from(e)));
                                break;
                            }
                        }
                    }
                    this.buffer.clear();
                    if let Some(cmd) = this.command_queue.pop_front() {
                        tracing::debug!("Yielding command after line parse");
                        return Poll::Ready(Some(cmd));
                    }
                    // If we hit a macro, we are now in ExpandingMacro state
                    if let AsyncParserState::ExpandingMacro { .. } = &this.state {
                        continue;
                    }
                }
                AsyncParserState::ExpandingMacro { fut, macro_name, macro_args, macro_span } => {
                    tracing::debug!("State: ExpandingMacro");
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Some(expanded)) => {
                            for cmd in expanded {
                                this.command_queue.push_back(Ok(cmd));
                            }
                            this.state = AsyncParserState::YieldingCommand;
                            continue;
                        }
                        Poll::Ready(None) => {
                            // Macro not found, yield as macro command
                            this.command_queue.push_back(Ok(OwnedGCodeCommand::Macro {
                                name: macro_name.clone(),
                                args: macro_args.clone(),
                                span: macro_span.clone(),
                            }));
                            this.state = AsyncParserState::YieldingCommand;
                            continue;
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    }
                }
                AsyncParserState::YieldingCommand => {
                    tracing::debug!("State: YieldingCommand");
                    this.state = AsyncParserState::ReadingLine;
                    continue;
                }
                AsyncParserState::Done => {
                    tracing::debug!("State: Done");
                    return Poll::Ready(None);
                }
                AsyncParserState::_Phantom(_) => unreachable!(),
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
                                }
                            }
                            Ok(cmd) => {
                                this.command_queue.push_back(Ok(OwnedGCodeCommand::from(cmd)));
                            }
                            Err(e) => {
                                this.command_queue.push_back(Err(OwnedGCodeError::from(e)));
                                break;
                            }
                        }
                    }
                    this.buffer.clear();
                    if let Some(cmd) = this.command_queue.pop_front() {
                        tracing::debug!("Yielding command after line parse");
                        return Poll::Ready(Some(cmd));
                    }
                    // If we hit a macro, we are now in ExpandingMacro state
                    if let AsyncParserState::ExpandingMacro { .. } = &this.state {
                        continue;
                    }
                }
                AsyncParserState::ExpandingMacro { fut, macro_name, macro_args, macro_span } => {
                    tracing::debug!("State: ExpandingMacro");
                    match fut.as_mut().poll(cx) {
                        Poll::Ready(Some(expanded)) => {
                            for cmd in expanded {
                                this.command_queue.push_back(Ok(cmd));
                            }
                            this.state = AsyncParserState::YieldingCommand;
                            continue;
                        }
                        Poll::Ready(None) => {
                            // Macro not found, yield as macro command
                            this.command_queue.push_back(Ok(OwnedGCodeCommand::Macro {
                                name: macro_name.clone(),
                                args: macro_args.clone(),
                                span: macro_span.clone(),
                            }));
                            this.state = AsyncParserState::YieldingCommand;
                            continue;
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    }
                }
                AsyncParserState::YieldingCommand => {
                    tracing::debug!("State: YieldingCommand");
                    this.state = AsyncParserState::ReadingLine;
                    continue;
                }
                AsyncParserState::Done => {
                    tracing::debug!("State: Done");
                    return Poll::Ready(None);
                }
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
