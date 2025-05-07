// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use std::{collections::HashMap, mem, ops::Range};

use bril::{
    builder::BasicBlockIdx,
    ir::{
        Function, FunctionItem, FunctionPrototype, Instruction, Label, LabelIdx,
    },
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

#[derive(Debug, Default, Clone)]
pub struct BasicBlock<'program> {
    pub label: Option<Label<'program>>,
    /// This includes the exit instruction if one is present (e.g., a
    /// `br` or `jmp`).
    pub instructions: &'program [Instruction],
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

#[derive(Debug)]
pub struct Cfg<'program> {
    pub prototype: FunctionPrototype,
    pub entry: BasicBlockIdx,
    pub vertices: SlotMap<BasicBlockIdx, BasicBlock<'program>>,
    pub edges: SecondaryMap<BasicBlockIdx, Exit>,
    pub rev_edges: SecondaryMap<BasicBlockIdx, Vec<BasicBlockIdx>>,
}

impl Cfg<'_> {
    pub fn successors(&self, current: BasicBlockIdx) -> Vec<BasicBlockIdx> {
        match &self.edges[current] {
            Exit::Unconditional(successor) => vec![*successor],
            Exit::Conditional { if_true, if_false } => {
                vec![*if_true, *if_false]
            }
            Exit::Return => vec![],
        }
    }
    pub fn predecessors(&self, current: BasicBlockIdx) -> Vec<BasicBlockIdx> {
        self.rev_edges[current].clone()
    }
}

pub struct CfgBuilder<'a, 'program> {
    cfg: Cfg<'program>,
    function: &'a Function<'program>,
    /// Whether the entry point to the CFG has been initialized (in
    /// `cfg.entry`).
    entry_is_init: bool,
    current_block: BasicBlock<'program>,
    current_range: Range<usize>,
    labels_to_blocks: HashMap<LabelIdx, BasicBlockIdx>,
    previous_idx: Option<BasicBlockIdx>,
    input_block_order: SecondaryMap<BasicBlockIdx, BasicBlockIdx>,
}

impl<'a, 'program> CfgBuilder<'a, 'program> {
    pub fn new(function: &'a Function<'program>) -> Self {
        let cfg = Cfg {
            prototype: function.prototype(),
            entry: BasicBlockIdx::default(),
            vertices: SlotMap::default(),
            edges: SecondaryMap::default(),
            rev_edges: SecondaryMap::default(),
        };

        Self {
            cfg,
            entry_is_init: false,
            function,
            current_block: BasicBlock::default(),
            current_range: 0..0,
            labels_to_blocks: HashMap::default(),
            previous_idx: None,
            input_block_order: SecondaryMap::new(),
        }
    }

    pub fn add_to_current(&mut self) {
        self.current_range.end += 1;
    }

    pub fn set_current_label(&mut self, idx: LabelIdx) {
        self.current_block.label = Some(
            self.function
                .labels
                .iter()
                .find(|label| label.idx == idx)
                .copied()
                .expect("label not found in function scope"),
        );
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
        let current_label = self.current_block.label;
        let mut current_block = mem::take(&mut self.current_block);
        current_block.instructions =
            &self.function.instructions[self.current_range.clone()];

        let block_idx = self.cfg.vertices.insert(current_block);
        self.current_range = self.current_range.end..self.current_range.end;

        if !self.entry_is_init {
            self.cfg.entry = block_idx;
            self.entry_is_init = true;
        }

        if let Some(previous_idx) = self.previous_idx {
            self.input_block_order.insert(previous_idx, block_idx);
        }
        self.previous_idx = Some(block_idx);

        if let Some(label) = current_label {
            self.labels_to_blocks.insert(label.idx, block_idx);
        }
    }

    pub fn finish(mut self) -> Cfg<'program> {
        for (block_idx, block) in &self.cfg.vertices {
            let mut successors = vec![];
            match &block.exit {
                LabeledExit::Fallthrough => {
                    let after_idx_opt =
                        self.input_block_order.get(block_idx).copied();
                    let exit = after_idx_opt
                        .map(Exit::Unconditional)
                        .unwrap_or(Exit::Return);
                    self.cfg.edges.insert(block_idx, exit);
                    if let Some(after_idx) = after_idx_opt {
                        successors.push(after_idx);
                    }
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
                    successors.push(destination_index);
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
                    successors.extend(vec![if_true_idx, if_false_idx]);
                }
                LabeledExit::Return => {
                    self.cfg.edges.insert(block_idx, Exit::Return);
                }
            }
            for successor in successors {
                self.cfg
                    .rev_edges
                    .entry(successor)
                    .unwrap()
                    .or_default()
                    .push(block_idx);
            }
        }

        self.cfg
    }
}

pub fn build_cfg<'program>(function: &Function<'program>) -> Cfg<'program> {
    let mut builder = CfgBuilder::new(function);

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
                if !builder.current_range.is_empty()
                    || builder.current_block.label.is_some()
                {
                    builder.finish_current_and_start_new_block();
                }
                builder.set_current_label(label_idx);
            }
        }
    }

    // handle trailing block
    if !builder.current_range.is_empty()
        || builder.current_block.label.is_some()
    {
        builder.finish_current_and_start_new_block();
    }

    builder.finish()
}
