// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use crate::{
    builder::{BasicBlockBuilder, FunctionBuilder, ProgramBuilder},
    ir,
};
use bril_rs;
use std::collections::HashMap;

pub fn flattened_program_repr(input: bril_rs::Program) -> ir::Program {
    let mut program_builder = ProgramBuilder::new();
    for function in &input.functions {
        let mut fn_builder =
            program_builder.new_function(function.name.clone());
        build_fn(&mut fn_builder, function);
        fn_builder.finish()
    }
    program_builder.finish()
}

fn basic_block_split(
    instrs: &[bril_rs::Code],
) -> impl Iterator<Item = &[bril_rs::Code]> {
    let mut basic_blocks = vec![];
    let mut haystack = instrs;
    if !matches!(instrs.first().unwrap(), bril_rs::Code::Label { .. }) {
        if let Some(next_label) = instrs
            .iter()
            .position(|instr| matches!(instr, bril_rs::Code::Label { .. }))
        {
            basic_blocks.push(&instrs[..next_label]);
            haystack = &instrs[next_label..];
        }
    }
    while !haystack.is_empty() {
        if let Some(next_label) = haystack[1..]
            .iter()
            .position(|instr| matches!(instr, bril_rs::Code::Label { .. }))
        {
            basic_blocks.push(&haystack[..=next_label]);
            haystack = &haystack[next_label + 1..];
        } else {
            basic_blocks.push(haystack);
            break;
        }
    }
    basic_blocks.into_iter()
}

fn build_fn(fn_builder: &mut FunctionBuilder<'_>, input: &bril_rs::Function) {
    let mut instr_builder = InstrBuilder::with_args(&input.args);
    if let Some(return_type) = &input.return_type {
        fn_builder.return_type(match return_type {
            bril_rs::Type::Int => ir::Type::Int,
            bril_rs::Type::Bool => ir::Type::Bool,
        });
    }
    fn_builder.parameters(
        &input
            .args
            .iter()
            .map(|arg| {
                instr_builder
                    .var_map
                    .get(arg.name.as_str())
                    .copied()
                    .unwrap()
            })
            .collect::<Vec<_>>(),
    );

    for instrs in basic_block_split(&input.instrs) {
        let mut block_builder = BasicBlockBuilder::new();

        let mut instrs = instrs.iter().peekable();
        instrs.next_if(|instr| {
            if let bril_rs::Code::Label { label } = instr {
                block_builder.label(label.clone());
                true
            } else {
                false
            }
        });

        for instr in instrs {
            if let bril_rs::Code::Instruction(instr) = instr {
                match instr_builder.translate(instr) {
                    Translated::Ok(instr) => block_builder.add_instr(instr),
                    Translated::ToResolve(instr, labels) => {
                        block_builder.add_patched_instr(instr, labels);
                    }
                }
            }
        }

        fn_builder.seal_block(block_builder);
    }
}

#[derive(Default)]
struct InstrBuilder<'a> {
    var_map: HashMap<&'a str, ir::Variable>,
    next_var: u32,
}

enum Translated {
    Ok(ir::Instruction),
    ToResolve(ir::Instruction, Vec<String>),
}

impl<'a> InstrBuilder<'a> {
    fn with_args(args: &'a [bril_rs::Argument]) -> Self {
        let mut next_var = 0;
        let var_map: HashMap<&str, ir::Variable> = args
            .iter()
            .map(|arg| {
                let ty = match arg.arg_type {
                    bril_rs::Type::Int => ir::Type::Int,
                    bril_rs::Type::Bool => ir::Type::Bool,
                };
                let arg_map = (arg.name.as_str(), ir::Variable(next_var, ty));
                next_var += 1;
                arg_map
            })
            .collect();
        Self { var_map, next_var }
    }

    fn translate(&mut self, instr: &'a bril_rs::Instruction) -> Translated {
        match instr {
            bril_rs::Instruction::Value {
                args,
                dest,
                op,
                funcs,
                op_type,
                ..
            } => {
                let args: Vec<_> = args
                    .iter()
                    .map(|arg| self.var_map.get(arg.as_str()).copied().unwrap())
                    .collect();
                let dest =
                    self.variable_or_next(dest.as_str(), op_type.clone());
                let translated = match op {
                    bril_rs::ValueOps::Add => {
                        ir::Instruction::Add(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Sub => {
                        ir::Instruction::Sub(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Mul => {
                        ir::Instruction::Mul(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Div => {
                        ir::Instruction::Div(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Eq => {
                        ir::Instruction::Eq(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Lt => {
                        ir::Instruction::Lt(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Gt => {
                        ir::Instruction::Gt(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Le => {
                        ir::Instruction::Le(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Ge => {
                        ir::Instruction::Ge(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::And => {
                        ir::Instruction::And(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Or => {
                        ir::Instruction::Or(dest, args[0], args[1])
                    }
                    bril_rs::ValueOps::Not => {
                        ir::Instruction::Not(dest, args[0])
                    }
                    bril_rs::ValueOps::Id => ir::Instruction::Id(dest, args[0]),
                    bril_rs::ValueOps::Call => {
                        return Translated::ToResolve(
                            ir::Instruction::Call(
                                Some(dest),
                                ir::FunctionIdx::UNDEF,
                                args.into_boxed_slice(),
                            ),
                            funcs.clone(),
                        );
                    }
                };
                Translated::Ok(translated)
            }
            bril_rs::Instruction::Constant {
                dest,
                op,
                value,
                const_type,
                ..
            } => {
                let dest =
                    self.variable_or_next(dest.as_str(), const_type.clone());
                let value = match value {
                    bril_rs::Literal::Int(val) => ir::Value::Int(*val),
                    bril_rs::Literal::Bool(val) => ir::Value::Bool(*val),
                };
                match op {
                    bril_rs::ConstOps::Const => {
                        Translated::Ok(ir::Instruction::Const(dest, value))
                    }
                }
            }
            bril_rs::Instruction::Effect {
                args,
                labels,
                funcs,
                op,
            } => {
                let args: Vec<_> = args
                    .iter()
                    .map(|arg| self.var_map.get(arg.as_str()).copied().unwrap())
                    .collect();
                let unresolved = ir::LabelIdx::UNDEF;
                match op {
                    bril_rs::EffectOps::Jump => Translated::ToResolve(
                        ir::Instruction::Jmp(unresolved),
                        labels.clone(),
                    ),
                    bril_rs::EffectOps::Branch => Translated::ToResolve(
                        ir::Instruction::Br(args[0], unresolved, unresolved),
                        labels.clone(),
                    ),
                    bril_rs::EffectOps::Print => Translated::Ok(
                        ir::Instruction::Print(args.into_boxed_slice()),
                    ),
                    bril_rs::EffectOps::Return => Translated::Ok(
                        ir::Instruction::Ret(args.first().copied()),
                    ),
                    bril_rs::EffectOps::Call => Translated::ToResolve(
                        ir::Instruction::Call(
                            None,
                            ir::FunctionIdx::UNDEF,
                            args.into_boxed_slice(),
                        ),
                        funcs.clone(),
                    ),
                    _ => panic!("unsupported effect code: {op}"),
                }
            }
        }
    }

    fn variable_or_next(
        &mut self,
        lit: &'a str,
        ty: bril_rs::Type,
    ) -> ir::Variable {
        let ir_ty = match ty {
            bril_rs::Type::Int => ir::Type::Int,
            bril_rs::Type::Bool => ir::Type::Bool,
        };
        if let Some(variable) = self.var_map.get(lit) {
            if variable.1 == ir_ty {
                return *variable;
            }
        }
        let next_var = ir::Variable(self.next_var, ir_ty);
        self.next_var += 1;
        self.var_map.insert(lit, next_var);
        next_var
    }
}
