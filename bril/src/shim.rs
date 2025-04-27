use crate::{
    builder::{BasicBlockBuilder, BasicBlockIdx, FunctionBuilder},
    ir,
};
use bril_rs;
use std::collections::HashMap;

pub fn flattened_program_repr(input: bril_rs::Program) -> ir::Program {
    let mut program = ir::Program::default();
    for function in &input.functions {
        let builder = FunctionBuilder::new(function.name.clone(), &mut program);
        build_fn(builder, function);
    }
    program
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

fn build_fn(mut fn_builder: FunctionBuilder<'_>, input: &bril_rs::Function) {
    let mut instr_builder = InstrBuilder::with_args(&input.args);
    let mut labels_to_resolve: HashMap<BasicBlockIdx, Vec<String>> =
        HashMap::new();
    let mut label_name2idx: HashMap<String, BasicBlockIdx> = HashMap::new();

    for instrs in basic_block_split(&input.instrs) {
        let mut block_builder = BasicBlockBuilder::new();
        let mut to_resolve = None;
        let mut block_label_name = None;

        let mut instrs = instrs.iter().peekable();
        instrs.next_if(|instr| {
            if let bril_rs::Code::Label { label } = instr {
                block_label_name = Some(label.clone());
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
                        to_resolve = Some(labels);
                        block_builder.add_instr(instr);
                    }
                }
            }
        }

        let block_idx = fn_builder.seal_block(block_builder);
        if let Some(labels) = to_resolve {
            labels_to_resolve.insert(block_idx, labels);
        }
        if let Some(block_label_name) = block_label_name {
            label_name2idx.insert(block_label_name, block_idx);
        }
    }

    for (block_idx, to_resolve) in labels_to_resolve {
        let label_idx: Vec<_> = to_resolve
            .into_iter()
            .map(|label_name| {
                let block_idx =
                    label_name2idx.get(&label_name).copied().unwrap_or_else(
                        || panic!("label: {label_name} does not exist"),
                    );
                fn_builder.block_label(block_idx)
            })
            .collect();
        match fn_builder.block_tail_mut(block_idx) {
            ir::Instruction::Jmp(dest) => *dest = label_idx[0],
            ir::Instruction::Br(_, true_br, false_br) => {
                *true_br = label_idx[0];
                *false_br = label_idx[1];
            }
            _ => unreachable!(),
        }
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
                let arg_map = (arg.name.as_str(), ir::Variable(next_var));
                next_var += 1;
                arg_map
            })
            .collect();
        Self { var_map, next_var }
    }

    fn translate(&mut self, instr: &'a bril_rs::Instruction) -> Translated {
        match instr {
            bril_rs::Instruction::Value { args, dest, op, .. } => {
                let args: Vec<_> =
                    args.iter().map(|arg| self.variable_or_next(arg)).collect();
                let dest = self.variable_or_next(dest.as_str());
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
                    _ => unreachable!("unsupported op code"),
                };
                Translated::Ok(translated)
            }
            bril_rs::Instruction::Constant {
                dest, op, value, ..
            } => {
                let dest = self.variable_or_next(dest.as_str());
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
                funcs: _funcs,
                op,
            } => {
                let args: Vec<_> =
                    args.iter().map(|arg| self.variable_or_next(arg)).collect();
                let unresolved = ir::LabelIdx(u32::MAX);
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
                    _ => unreachable!(),
                }
            }
        }
    }

    fn variable_or_next(&mut self, lit: &'a str) -> ir::Variable {
        *self.var_map.entry(lit).or_insert_with(|| {
            let next_var = ir::Variable(self.next_var);
            self.next_var += 1;
            next_var
        })
    }
}
