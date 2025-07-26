//! Shared trait for executing parsed G-code commands in any environment

use crate::gcode::GCodeCommand;

pub trait GCodeExecutor: Send {
    fn execute(&mut self, cmd: GCodeCommand<'static>);
}
