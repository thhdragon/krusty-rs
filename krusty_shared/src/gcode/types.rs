//! Types and traits re-exported from the original gcode.rs for shared use

use std::ops::Range;

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

#[async_trait::async_trait]
pub trait MacroExpander: Send + Sync {
    async fn expand(&self, name: String, args: String) -> Option<Vec<OwnedGCodeCommand>>;
}

#[derive(Debug, Clone)]
pub struct MacroProcessor;

impl MacroProcessor {
    pub fn new() -> Self { Self }
}

impl MacroProcessor {
    /// Parse and expand a G-code command string asynchronously, returning a Vec of Result<OwnedGCodeCommand, OwnedGCodeError>.
    pub async fn parse_and_expand_async_owned(&self, command: &str) -> Vec<Result<OwnedGCodeCommand, OwnedGCodeError>> {
        // For demonstration, this is a stub. In a real implementation, this would parse the command,
        // expand macros if present, and return the expanded commands or errors.
        // Here, we just parse a single command and return it as an OwnedGCodeCommand.
        // TODO: Replace with full parser and macro expansion logic.
        use crate::gcode::{GCodeCommand, OwnedGCodeCommand, GCodeError, OwnedGCodeError};
        let mut results = Vec::new();
        // Example: parse a single word command (e.g., G1 X10 Y20)
        if command.trim().is_empty() {
            return results;
        }
        // This is a placeholder parser. Replace with real parser logic.
        let span = GCodeSpan { range: 0..command.len() };
        let cmd = GCodeCommand::Word { letter: command.chars().next().unwrap_or('G'), value: &command[1..], span: span.clone() };
        results.push(Ok(OwnedGCodeCommand::from(cmd)));
        results
    }
}
