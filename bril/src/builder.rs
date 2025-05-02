use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::ops::Range;

use crate::ir::{
    FunctionInternal, Instruction, LabelIdx, Program, Type, Variable,
};

enum PatchType {
    Label(Vec<String>),
    Func(String),
}
struct Patch {
    offset: usize,
    ty: PatchType,
}

#[derive(Default)]
pub struct ProgramBuilder {
    program: Program,
    patches: Vec<Patch>,
}

impl ProgramBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_function(&mut self, name: String) -> FunctionBuilder {
        FunctionBuilder {
            name,
            blocks: SlotMap::with_key(),
            block_names: SecondaryMap::new(),
            block_name_id: 0,
            program_builder: self,
            patches: vec![],
            parameters: vec![],
            return_type: None,
        }
    }

    pub fn finish(mut self) -> Program {
        for patch in self.patches {
            if let PatchType::Func(function_name) = patch.ty {
                let function_idx = self
                    .program
                    .find_function_symbol(&function_name)
                    .unwrap_or_else(|| {
                        panic!("function '{function_name}' not found")
                    });
                match &mut self.program.instructions[patch.offset] {
                    Instruction::Call(_, callee, _) => *callee = function_idx,
                    _ => unreachable!("only call instr can be patched as Func"),
                }
            } else {
                unreachable!("unresolved label patch")
            }
        }
        self.program
    }
}

new_key_type! { pub struct BasicBlockIdx; }
pub struct FunctionBuilder<'program> {
    name: String,
    blocks: SlotMap<BasicBlockIdx, Range<usize>>,
    block_names: SecondaryMap<BasicBlockIdx, LabelIdx>,
    block_name_id: usize,
    program_builder: &'program mut ProgramBuilder,
    patches: Vec<Patch>,
    parameters: Vec<Variable>,
    return_type: Option<Type>,
}

impl<'program> FunctionBuilder<'program> {
    pub fn parameters(&mut self, parameters: &[Variable]) {
        self.parameters.extend_from_slice(parameters);
    }

    pub fn return_type(&mut self, ty: Type) {
        self.return_type = Some(ty);
    }

    pub fn block_mut(&mut self, idx: BasicBlockIdx) -> &mut [Instruction] {
        &mut self.program_builder.program.instructions[self.blocks[idx].clone()]
    }

    pub fn block_label(&self, idx: BasicBlockIdx) -> LabelIdx {
        self.block_names[idx]
    }

    pub fn block_tail_mut(&mut self, idx: BasicBlockIdx) -> &mut Instruction {
        self.block_mut(idx).last_mut().unwrap()
    }

    pub fn seal_block(
        &mut self,
        block_builder: BasicBlockBuilder,
    ) -> BasicBlockIdx {
        let label_idx = self.program_builder.program.add_label(
            block_builder.label.unwrap_or_else(|| {
                let default_label_name = format!("L{}", self.block_name_id);
                self.block_name_id += 1;
                default_label_name
            }),
        );

        let start = self.program_builder.program.instructions.len();
        let block_idx = self
            .blocks
            .insert(start..start + block_builder.instrs.len());
        self.block_names.insert(block_idx, label_idx);

        self.program_builder
            .program
            .instructions
            .extend_from_slice(&block_builder.instrs);

        // offset in patches is offset in program's instruction buffer
        self.patches.extend(block_builder.patches.into_iter().map(
            |mut patch| {
                patch.offset += start;
                patch
            },
        ));
        block_idx
    }

    pub fn finish(self) {
        let num_instrs = self
            .blocks
            .values()
            .fold(0, |acc, range| acc + range.end - range.start);
        let end = self.program_builder.program.instructions.len();
        let start = end - num_instrs;

        let name = self.program_builder.program.add_string(self.name);
        self.program_builder.program.add_function(FunctionInternal {
            name,
            range: start..end,
            parameters: self.parameters,
            labels: self.block_names.values().copied().collect(),
            return_type: self.return_type,
        });

        for patch in self.patches {
            match patch.ty {
                PatchType::Label(labels) => {
                    let resolved_label_idx: Vec<_> = labels
                        .iter()
                        .map(|name| {
                            self.block_names
                                .values()
                                .find(|idx| {
                                    self.program_builder
                                        .program
                                        .get_label_name(**idx)
                                        == name
                                })
                                .copied()
                                .unwrap_or_else(|| {
                                    panic!("label '{name}' not found")
                                })
                        })
                        .collect();

                    let instructions =
                        &mut self.program_builder.program.instructions;
                    match &mut instructions[patch.offset] {
                        Instruction::Jmp(dest) => *dest = resolved_label_idx[0],
                        Instruction::Br(_, if_true, if_false) => {
                            *if_true = resolved_label_idx[0];
                            *if_false = resolved_label_idx[1];
                        }
                        _ => unreachable!(),
                    }
                }
                PatchType::Func(_) => self.program_builder.patches.push(patch),
            }
        }
    }
}

#[derive(Default)]
pub struct BasicBlockBuilder {
    pub instrs: Vec<Instruction>,
    label: Option<String>,
    patches: Vec<Patch>,
}

impl BasicBlockBuilder {
    pub fn new() -> Self {
        Self {
            instrs: vec![],
            label: None,
            patches: vec![],
        }
    }

    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            instrs: vec![],
            label: Some(label.into()),
            patches: vec![],
        }
    }

    pub fn add_instr(&mut self, instr: Instruction) {
        self.instrs.push(instr)
    }

    pub fn add_patched_instr(
        &mut self,
        instr: Instruction,
        symbols: Vec<String>,
    ) {
        match &instr {
            Instruction::Br(_, _, _) | Instruction::Jmp(_) => {
                self.patches.push(Patch {
                    offset: self.instrs.len(),
                    ty: PatchType::Label(symbols),
                })
            }
            Instruction::Call(_, _, _) => self.patches.push(Patch {
                offset: self.instrs.len(),
                ty: PatchType::Func(symbols[0].clone()),
            }),
            _ => unreachable!("only jmp, br, call instr can be patched"),
        }
        self.instrs.push(instr);
    }

    pub fn label(&mut self, name: String) {
        self.label = Some(name)
    }

    pub fn is_empty(&self) -> bool {
        self.instrs.is_empty() && self.label.is_none()
    }
}
