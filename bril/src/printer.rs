use std::{fmt, ops::Range};

use crate::ir::{Function, LabelIdx, Program, StringIdx};

pub struct Printer<'formatter, W: fmt::Write> {
    f: &'formatter mut W,
}

impl<'formatter, W: fmt::Write> Printer<'formatter, W> {
    pub fn new(f: &'formatter mut W) -> Self {
        Self { f }
    }

    pub fn print_program(&mut self, program: &Program) -> fmt::Result {
        for function in program.functions() {
            self.print_function(function)?;
        }
        Ok(())
    }

    pub fn print_function(&mut self, function: Function) -> fmt::Result {
        writeln!(self.f, "@{} {{", function.name)?;
        for (index, instruction) in function.instructions.iter().enumerate() {}
        write!(self.f, "}}")
    }
}
