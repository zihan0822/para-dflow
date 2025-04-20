use slotmap::{SecondaryMap, SlotMap, new_key_type};

use crate::ir::{Instruction, LabelIdx, Program};

new_key_type! { pub struct BasicBlockIdx; }
pub struct FunctionBuilder<'program> {
    blocks: SlotMap<BasicBlockIdx, Vec<Instruction>>,
    block_names: SecondaryMap<BasicBlockIdx, LabelIdx>,
    block_name_id: usize,
    entry_block_idx: BasicBlockIdx,
    program: &'program mut Program,
}

impl<'program> FunctionBuilder<'program> {
    pub fn new(program: &'program mut Program) -> Self {
        let mut blocks = SlotMap::with_key();
        let entry_block_idx = blocks.insert(vec![]);
        Self {
            blocks,
            entry_block_idx,
            block_name_id: 0,
            block_names: SecondaryMap::new(),
            program,
        }
    }

    pub fn entry_block_idx(&self) -> BasicBlockIdx {
        self.entry_block_idx
    }

    pub fn new_block(&mut self) -> BasicBlockIdx {
        let block_idx = self.blocks.insert(vec![]);
        let block_name_label_idx =
            self.program.add_label(format!("L{}", self.block_name_id));
        self.block_names.insert(block_idx, block_name_label_idx);
        self.block_name_id += 1;
        block_idx
    }

    pub fn block_mut(&mut self, idx: BasicBlockIdx) -> &mut Vec<Instruction> {
        self.blocks.get_mut(idx).unwrap()
    }

    pub fn block_label(&self, idx: BasicBlockIdx) -> LabelIdx {
        self.block_names[idx]
    }
}
