// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use fixedbitset::FixedBitSet;
use std::collections::VecDeque;

use bril::builder::BasicBlockIdx;
use slotmap::SecondaryMap;

use crate::{Direction, TraverseCfgLike, construct_postorder};

pub fn solve_dataflow<'a, C: TraverseCfgLike<'a>>(
    cfg_like: &C,
    context: &C::Context,
    direction: Direction,
    entry_inputs: FixedBitSet,
    merge: impl Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet,
    transfer: impl Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet,
) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let postorder_traversal = construct_postorder(cfg_like, context);
    let mut solution =
        SecondaryMap::with_capacity(cfg_like.vertices_capacity());
    for &block_idx in &postorder_traversal {
        solution.insert(block_idx, FixedBitSet::new());
    }

    let mut blocks = match direction {
        Direction::Forward => {
            VecDeque::from_iter(postorder_traversal.into_iter().rev())
        }
        Direction::Backward => VecDeque::from_iter(postorder_traversal),
    };

    let mut initial_in = entry_inputs.clone();
    let entry = blocks[0];
    while let Some(current) = blocks.pop_front() {
        match direction {
            Direction::Forward => {
                let mut to_merge = vec![];
                if current == entry {
                    to_merge.push(entry_inputs.clone());
                }
                for predecessor in cfg_like.predecessors(context, current) {
                    to_merge.push(solution[predecessor].clone());
                }
                initial_in = to_merge
                    .into_iter()
                    .reduce(|in1, in2| merge(in1, &in2))
                    .unwrap();
            }
            Direction::Backward => {
                for predecessor in cfg_like.successors(context, current) {
                    initial_in = merge(initial_in, &solution[predecessor]);
                }
            }
        }

        let new_out = transfer(current, initial_in.clone());
        if !new_out.eq(&solution[current]) {
            solution[current] = new_out;
            match direction {
                Direction::Forward => {
                    blocks.extend(cfg_like.successors(context, current));
                }
                Direction::Backward => {
                    blocks.extend(cfg_like.predecessors(context, current));
                }
            }
        }

        // if current == entry {
        //     initial_in = entry_inputs.clone();
        // } else {
        //     initial_in = FixedBitSet::new();
        // }
    }
    solution
}
