// Tests for the advanced G-code parser prototype (moved from src/gcode/parser_tests.rs)

use krusty_rs::gcode::parser::*;
use async_trait::async_trait;
use futures_util::io::Cursor;
use futures_util::stream::StreamExt;
use krusty_rs::print_job::PrintJobManager;
use krusty_rs::gcode::parser::GCodeCommand;

// ...existing code from parser_tests.rs...
// (Insert all #[test] and #[tokio::test] functions here, as previously cataloged)
