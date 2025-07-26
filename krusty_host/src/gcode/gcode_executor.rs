//! Trait for executing parsed G-code commands in a real-world system.

pub trait GCodeExecutor: Send {
    fn execute(&mut self, cmd: crate::gcode::parser::GCodeCommand<'static>);
}
