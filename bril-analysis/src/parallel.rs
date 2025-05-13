// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use fixedbitset::FixedBitSet;
use std::collections::VecDeque;

use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::SecondaryMap;

use crate::{construct_postorder, Direction};

pub fn solve_dataflow(
    cfg: &Cfg,
    direction: Direction,
    entry_inputs: FixedBitSet,
    merge: impl Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet,
    transfer: impl Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet,
) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let postorder_traversal = construct_postorder(cfg);
    let blocks = match direction {
        Direction::Forward => {
            VecDeque::from_iter(postorder_traversal.into_iter().rev())
        }
        Direction::Backward => VecDeque::from_iter(postorder_traversal),
    };

    todo!()
}
