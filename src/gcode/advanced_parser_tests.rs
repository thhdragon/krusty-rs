//! Tests for the advanced G-code parser prototype

use super::advanced_parser::*;

#[test]
fn test_word_parsing() {
    let src = "G1 X10.0 Y20.0";
    let mut parser = GCodeParser::new(src, GCodeParserConfig { enable_comments: false, ..Default::default() });
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G word"),
    }
    let cmd2 = parser.next_command().unwrap().unwrap();
    match cmd2 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'X');
            assert_eq!(value, "10.0");
        },
        _ => panic!("Expected X word"),
    }
    let cmd3 = parser.next_command().unwrap().unwrap();
    match cmd3 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'Y');
            assert_eq!(value, "20.0");
        },
        _ => panic!("Expected Y word"),
    }
    assert!(parser.next_command().is_none());
}

#[test]
fn test_comment_parsing() {
    let src = "; this is a comment\nG1";
    let mut parser = GCodeParser::new(src, GCodeParserConfig { enable_comments: true, ..Default::default() });
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Comment(comment, _) => {
            assert_eq!(comment, "this is a comment");
        },
        _ => panic!("Expected comment"),
    }
    let cmd2 = parser.next_command().unwrap().unwrap();
    match cmd2 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G word after comment"),
    }
    assert!(parser.next_command().is_none());
}

#[test]
fn test_macro_parsing() {
    let src = "{macro_name arg1 arg2} G1";
    let mut parser = GCodeParser::new(src, GCodeParserConfig { enable_macros: true, ..Default::default() });
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Macro { name, args, .. } => {
            assert_eq!(name, "macro_name");
            assert_eq!(args, "arg1 arg2");
        },
        _ => panic!("Expected macro"),
    }
    let cmd2 = parser.next_command().unwrap().unwrap();
    match cmd2 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G word after macro"),
    }
    assert!(parser.next_command().is_none());
}

#[test]
fn test_error_handling() {
    let src = "G1 $ X10";
    let mut parser = GCodeParser::new(src, GCodeParserConfig::default());
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G word"),
    }
    let cmd2 = parser.next_command().unwrap();
    assert!(cmd2.is_err());
    let cmd3 = parser.next_command().unwrap().unwrap();
    match cmd3 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'X');
            assert_eq!(value, "10");
        },
        _ => panic!("Expected X word"),
    }
    assert!(parser.next_command().is_none());
}

#[test]
fn test_macro_expansion() {
    struct DummyExpander;
    impl<'a> MacroExpander<'a> for DummyExpander {
        fn expand(&self, name: &'a str, args: &'a str) -> Option<Vec<GCodeCommand<'a>>> {
            if name == "repeat" && args == "G1 X1" {
                Some(vec![
                    GCodeCommand::Word { letter: 'G', value: "1", span: GCodeSpan { range: 0..1 } },
                    GCodeCommand::Word { letter: 'X', value: "1", span: GCodeSpan { range: 0..1 } },
                ])
            } else {
                None
            }
        }
    }
    let src = "{repeat G1 X1} G2";
    let mut parser = GCodeParser::new(src, GCodeParserConfig { enable_macros: true, ..Default::default() })
        .with_macro_expander(&DummyExpander);
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G1 from macro expansion"),
    }
    let cmd2 = parser.next_command().unwrap().unwrap();
    match cmd2 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'X');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected X1 from macro expansion"),
    }
    let cmd3 = parser.next_command().unwrap().unwrap();
    match cmd3 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "2");
        },
        _ => panic!("Expected G2 after macro expansion"),
    }
    assert!(parser.next_command().is_none());
}

#[test]
fn test_error_recovery() {
    let src = "G1 $ X10 ;comment\nG2";
    let mut parser = GCodeParser::new(src, GCodeParserConfig { enable_comments: true, ..Default::default() });
    // G1
    let cmd1 = parser.next_command().unwrap().unwrap();
    match cmd1 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "1");
        },
        _ => panic!("Expected G1"),
    }
    // $ (error)
    let cmd2 = parser.next_command().unwrap();
    assert!(cmd2.is_err());
    // X10
    let cmd3 = parser.next_command().unwrap().unwrap();
    match cmd3 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'X');
            assert_eq!(value, "10");
        },
        _ => panic!("Expected X10"),
    }
    // ;comment
    let cmd4 = parser.next_command().unwrap().unwrap();
    match cmd4 {
        GCodeCommand::Comment(comment, _) => {
            assert_eq!(comment, "comment");
        },
        _ => panic!("Expected comment"),
    }
    // G2
    let cmd5 = parser.next_command().unwrap().unwrap();
    match cmd5 {
        GCodeCommand::Word { letter, value, .. } => {
            assert_eq!(letter, 'G');
            assert_eq!(value, "2");
        },
        _ => panic!("Expected G2"),
    }
    assert!(parser.next_command().is_none());
}
