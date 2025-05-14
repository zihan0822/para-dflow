// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use hibitset::BitSet;
use std::collections::{HashMap, VecDeque};

use bril::builder::BasicBlockIdx;
use slotmap::SecondaryMap;

use crate::{Direction, TraverseCfgLike, construct_postorder};

pub fn solve_dataflow<'a, C: TraverseCfgLike<'a>>(
    cfg_like: &C,
    context: &C::Context,
    direction: Direction,
    entry_inputs: HashMap<BasicBlockIdx, BitSet>,
    merge: impl Fn(BitSet, &BitSet) -> BitSet,
    transfer: impl Fn(BasicBlockIdx, BitSet) -> BitSet,
) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let postorder_traversal = construct_postorder(cfg_like, context);
    let mut solution =
        SecondaryMap::with_capacity(cfg_like.vertices_capacity());
    for &block_idx in &postorder_traversal {
        solution.insert(block_idx, BitSet::new());
    }

    let mut blocks = match direction {
        Direction::Forward => {
            VecDeque::from_iter(postorder_traversal.into_iter().rev())
        }
        Direction::Backward => VecDeque::from_iter(postorder_traversal),
    };

    while let Some(current) = blocks.pop_front() {
        let mut initial_in =
            entry_inputs.get(&current).cloned().unwrap_or(BitSet::new());
        match direction {
            Direction::Forward => {
                for predecessor in cfg_like.predecessors(context, current) {
                    initial_in = merge(initial_in, &solution[predecessor]);
                }
            }
            Direction::Backward => {
                for predecessor in cfg_like.successors(context, current) {
                    initial_in = merge(initial_in, &solution[predecessor]);
                }
            }
        }

        let new_out = transfer(current, initial_in);
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
    }
    solution
}
