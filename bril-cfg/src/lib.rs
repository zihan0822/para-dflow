// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use std::{collections::HashMap, mem, ops::Range};

use bril::{
    builder::BasicBlockIdx,
    ir::{Function, FunctionItem, Instruction, LabelIdx},
};
use slotmap::{SecondaryMap, SlotMap};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum LabeledExit {
    #[default]
    Fallthrough,
    Unconditional(LabelIdx),
    Conditional {
        if_true: LabelIdx,
        if_false: LabelIdx,
    },
    Return,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct BasicBlock {
    pub label: Option<LabelIdx>,
    /// This range includes the exit instruction if one is present (e.g., a
    /// `br` or `jmp`).
    pub range: Range<usize>,
    pub is_entry: bool,
    pub exit: LabeledExit,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Exit {
    Unconditional(BasicBlockIdx),
    Conditional {
        if_true: BasicBlockIdx,
        if_false: BasicBlockIdx,
    },
    Return,
}

#[derive(Debug, Default)]
pub struct Cfg {
    pub entry: BasicBlockIdx,
    pub vertices: SlotMap<BasicBlockIdx, BasicBlock>,
    pub edges: SecondaryMap<BasicBlockIdx, Exit>,
}

#[derive(Debug, Default)]
pub struct CfgBuilder {
    cfg: Cfg,
    /// Whether the entry point to the CFG has been initialized (in
    /// `cfg.entry`).
    entry_is_init: bool,
    current_block: BasicBlock,
    labels_to_blocks: HashMap<LabelIdx, BasicBlockIdx>,
    previous_idx: Option<BasicBlockIdx>,
    input_block_order: SecondaryMap<BasicBlockIdx, BasicBlockIdx>,
}

impl CfgBuilder {
    pub fn add_to_current(&mut self) {
        self.current_block.range.end += 1;
    }

    pub fn set_current_label(&mut self, label: LabelIdx) {
        self.current_block.label = Some(label);
    }

    pub fn mark_current_as_entry(&mut self) {
        self.current_block.is_entry = true;
    }

    pub fn set_current_exit(&mut self, exit: LabeledExit) {
        assert!(
            exit != LabeledExit::Fallthrough,
            "fallthrough is inferred if no exit is specified"
        );
        self.current_block.exit = exit;
    }

    pub fn finish_current_and_start_new_block(&mut self) {
        let current_label_idx = self.current_block.label.clone();
        let current_block = mem::take(&mut self.current_block);
        self.current_block.range =
            current_block.range.end..current_block.range.end;
        let block_idx = self.cfg.vertices.insert(current_block);

        if !self.entry_is_init {
            self.cfg.entry = block_idx;
            self.entry_is_init = true;
        }

        if let Some(previous_idx) = self.previous_idx {
            self.input_block_order.insert(previous_idx, block_idx);
        }
        self.previous_idx = Some(block_idx);

        if let Some(label_idx) = current_label_idx {
            self.labels_to_blocks.insert(label_idx, block_idx);
        }
    }

    pub fn finish(mut self) -> Cfg {
        for (block_idx, block) in &self.cfg.vertices {
            match &block.exit {
                LabeledExit::Fallthrough => {
                    let after_idx_opt =
                        self.input_block_order.get(block_idx).copied();
                    let exit = after_idx_opt
                        .map(|after_idx| Exit::Unconditional(after_idx))
                        .unwrap_or(Exit::Return);
                    self.cfg.edges.insert(block_idx, exit);
                }
                LabeledExit::Unconditional(always) => {
                    let destination_index = *self
                        .labels_to_blocks
                        .get(always)
                        .expect("Unknown label in unconditional branch");
                    self.cfg.edges.insert(
                        block_idx,
                        Exit::Unconditional(destination_index),
                    );
                }
                LabeledExit::Conditional { if_true, if_false } => {
                    let if_true_idx = *self
                        .labels_to_blocks
                        .get(if_true)
                        .expect("Unknown label in if branch");
                    let if_false_idx = *self
                        .labels_to_blocks
                        .get(if_false)
                        .expect("Unknown label in else branch");
                    self.cfg.edges.insert(
                        block_idx,
                        Exit::Conditional {
                            if_true: if_true_idx,
                            if_false: if_false_idx,
                        },
                    );
                }
                LabeledExit::Return => {
                    self.cfg.edges.insert(block_idx, Exit::Return);
                }
            }
        }

        self.cfg
    }
}

pub fn build_cfg(function: &Function) -> Cfg {
    let mut builder = CfgBuilder::default();

    for item in function.items_iter() {
        match item {
            FunctionItem::Instruction(instruction) => match instruction {
                Instruction::Jmp(label_idx) => {
                    builder.add_to_current();

                    builder.set_current_exit(LabeledExit::Unconditional(
                        *label_idx,
                    ));

                    builder.finish_current_and_start_new_block();
                }
                Instruction::Br(_, if_true_label_idx, if_false_label_idx) => {
                    builder.add_to_current();

                    builder.set_current_exit(LabeledExit::Conditional {
                        if_true: *if_true_label_idx,
                        if_false: *if_false_label_idx,
                    });

                    builder.finish_current_and_start_new_block();
                }
                Instruction::Ret(_) => {
                    builder.add_to_current();

                    builder.set_current_exit(LabeledExit::Return);

                    builder.finish_current_and_start_new_block();
                }
                _ => {
                    builder.add_to_current();
                }
            },
            FunctionItem::Label(label_idx) => {
                if !builder.current_block.range.is_empty()
                    || builder.current_block.label.is_some()
                {
                    builder.finish_current_and_start_new_block();
                }
                builder.set_current_label(label_idx);
            }
        }
    }

    builder.finish()
}
