//! Tests for the advanced G-code parser prototype

use async_trait::async_trait;
use futures_util::io::Cursor;
use futures_util::stream::StreamExt;
use krusty_rs::print_job::PrintJobManager;
use krusty_rs::gcode::parser::*;

struct DummyExpander;

#[async_trait]
impl MacroExpander for DummyExpander {
    async fn expand(&self, name: String, args: String) -> Option<Vec<OwnedGCodeCommand>> {
        if name == "repeat" && args == "G1 X1" {
            Some(vec![
                OwnedGCodeCommand::Word { letter: 'G', value: "1".to_string(), span: GCodeSpan { range: 0..1 } },
                OwnedGCodeCommand::Word { letter: 'X', value: "1".to_string(), span: GCodeSpan { range: 0..1 } },
            ])
        } else {
            None
        }
    }
}

// ...rest of the test functions from the original parser_tests.rs...
