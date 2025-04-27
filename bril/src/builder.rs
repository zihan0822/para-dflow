use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::ops::Range;

use crate::ir::{FunctionInternal, Instruction, LabelIdx, Program};

new_key_type! { pub struct BasicBlockIdx; }
pub struct FunctionBuilder<'program> {
    name: String,
    block_name_id: usize,
    blocks: SlotMap<BasicBlockIdx, Range<usize>>,
    block_names: SecondaryMap<BasicBlockIdx, LabelIdx>,
    program: &'program mut Program,
}

impl<'program> FunctionBuilder<'program> {
    pub fn new(name: String, program: &'program mut Program) -> Self {
        Self {
            name,
            blocks: SlotMap::with_key(),
            block_names: SecondaryMap::new(),
            block_name_id: 0,
            program,
        }
    }

    pub fn block_mut(&mut self, idx: BasicBlockIdx) -> &mut [Instruction] {
        &mut self.program.instructions[self.blocks[idx].clone()]
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
        let label_idx = if let Some(label) = block_builder.label {
            self.program.add_label(label)
        } else {
            let label_idx =
                self.program.add_label(format!("L{}", self.block_name_id));
            self.block_name_id += 1;
            label_idx
        };
        self.program
            .instructions
            .extend_from_slice(&block_builder.instrs);

        let start = self.program.get_label_offset(label_idx);
        let block_idx = self
            .blocks
            .insert(start..start + block_builder.instrs.len());
        self.block_names.insert(block_idx, label_idx);
        block_idx
    }

    pub fn finish(self) {
        let num_instrs = self
            .blocks
            .values()
            .fold(0, |acc, range| acc + range.end - range.start);
        let end = self.program.instructions.len();
        let name = self.program.add_string(self.name);
        self.program.add_function(FunctionInternal {
            name,
            range: (end - num_instrs)..end,
            parameters: vec![],
        });
    }
}

#[derive(Default)]
pub struct BasicBlockBuilder {
    pub instrs: Vec<Instruction>,
    label: Option<String>,
}

impl BasicBlockBuilder {
    pub fn new() -> Self {
        Self {
            instrs: vec![],
            label: None,
        }
    }

    pub fn add_instr(&mut self, instr: Instruction) {
        self.instrs.push(instr)
    }

    pub fn label(&mut self, name: String) {
        self.label = Some(name)
    }

    pub fn is_empty(&self) -> bool {
        self.instrs.is_empty() && self.label.is_none()
    }
}
