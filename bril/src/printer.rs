use std::fmt;

use crate::ir::{Function, Instruction, Program};

pub struct Printer<'formatter, W: fmt::Write> {
    f: &'formatter mut W,
}

impl<'formatter, W: fmt::Write> Printer<'formatter, W> {
    pub fn new(f: &'formatter mut W) -> Self {
        Self { f }
    }

    pub fn print_program(&mut self, program: &Program) -> fmt::Result {
        for function in program.functions() {
            self.print_function(program, function)?;
        }
        Ok(())
    }

    pub fn print_function(
        &mut self,
        program: &Program,
        function: Function,
    ) -> fmt::Result {
        let mut labels = function.labels.iter().peekable();
        let args = function
            .parameters
            .iter()
            .map(|arg| format!("x{}: {}", arg.0, arg.1))
            .collect::<Vec<_>>()
            .join(", ");
        let ret = function
            .return_type
            .map_or(String::from(""), |ty| format!(": {ty}"));

        writeln!(self.f, "@{}({}){} {{", function.name, args, ret)?;
        for (offset, instruction) in function.instructions.iter().enumerate() {
            while let Some((_, label_name)) =
                labels.next_if(|(label_offset, _)| *label_offset == offset)
            {
                writeln!(self.f, ".{label_name}:")?;
            }
            write!(self.f, "\t")?;
            self.print_instruction(program, instruction)?;
        }
        // handle the special case where labels are appended to the end of
        // instructions
        for &(offset, label_name) in labels {
            assert_eq!(offset, function.instructions.len());
            writeln!(self.f, ".{label_name}:")?;
        }
        write!(self.f, "}}")
    }

    fn print_instruction(
        &mut self,
        program: &Program,
        instruction: &Instruction,
    ) -> fmt::Result {
        match instruction {
            Instruction::Add(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = add x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Sub(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = sub x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Mul(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = mul x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Div(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = div x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Eq(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = eq x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Lt(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = lt x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Gt(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = gt x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Le(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = le x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Ge(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = ge x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Not(dest, arg0) => {
                writeln!(self.f, "x{}: {} = not x{};", dest.0, dest.1, arg0.0)?
            }
            Instruction::And(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = and x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Or(dest, arg0, arg1) => writeln!(
                self.f,
                "x{}: {} = or x{} x{};",
                dest.0, dest.1, arg0.0, arg1.0
            )?,
            Instruction::Jmp(label) => {
                writeln!(self.f, "jmp .{};", program.get_label_name(*label))?
            }
            Instruction::Br(condition, if_true, if_false) => writeln!(
                self.f,
                "br x{} .{} .{};",
                condition.0,
                program.get_label_name(*if_true),
                program.get_label_name(*if_false)
            )?,
            Instruction::Call(dest, function_idx, args) => {
                let function = program.get_function(*function_idx);
                let args = args
                    .iter()
                    .map(|arg| format!("x{}", arg.0))
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some(dest) = dest {
                    writeln!(
                        self.f,
                        "x{}: {} = call @{} {};",
                        dest.0, dest.1, function.name, args
                    )?;
                } else {
                    writeln!(self.f, "call @{} {};", function.name, args)?;
                }
            }
            Instruction::Ret(ret) => {
                if let Some(ret) = ret {
                    writeln!(self.f, "ret x{};", ret.0)?;
                } else {
                    writeln!(self.f, "ret;")?;
                }
            }
            Instruction::Const(dest, lit) => {
                writeln!(self.f, "x{}: {} = const {};", dest.0, dest.1, lit)?
            }
            Instruction::Id(dest, arg) => {
                writeln!(self.f, "x{}: {} = id x{};", dest.0, dest.1, arg.0)?
            }
            Instruction::Print(args) => {
                let args = args
                    .iter()
                    .map(|arg| format!("x{}", arg.0))
                    .collect::<Vec<_>>()
                    .join(" ");
                writeln!(self.f, "print {};", args)?;
            }
            Instruction::Nop => {}
        }
        Ok(())
    }
}
