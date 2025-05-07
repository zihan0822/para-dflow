// Copyright (C) 2025 Zihan Li and Ethan Uppal.
pub mod analysis;
pub mod scc;

use fixedbitset::FixedBitSet;
use std::collections::VecDeque;

use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::SecondaryMap;

pub enum Direction {
    Forward,
    Backward,
}

pub fn solve_dataflow(
    cfg: &Cfg,
    direction: Direction,
    entry_inputs: FixedBitSet,
    merge: impl Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet,
    transfer: impl Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet,
) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let postorder_traversal = construct_postorder(cfg);
    let mut blocks = match direction {
        Direction::Forward => {
            VecDeque::from_iter(postorder_traversal.into_iter().rev())
        }
        Direction::Backward => VecDeque::from_iter(postorder_traversal),
    };

    let mut solution = SecondaryMap::with_capacity(cfg.vertices.capacity());
    for block_idx in cfg.vertices.keys() {
        solution.insert(block_idx, FixedBitSet::new());
    }
    let mut initial_in = entry_inputs;
    while let Some(current) = blocks.pop_front() {
        match direction {
            Direction::Forward => {
                for predecessor in cfg.predecessors(current) {
                    initial_in = merge(initial_in, &solution[predecessor]);
                }
            }
            Direction::Backward => {
                for predecessor in cfg.successors(current) {
                    initial_in = merge(initial_in, &solution[predecessor]);
                }
            }
        }

        let new_out = transfer(current, initial_in);
        if !new_out.eq(&solution[current]) {
            solution[current] = new_out;
            match direction {
                Direction::Forward => {
                    blocks.extend(cfg.successors(current));
                }
                Direction::Backward => {
                    blocks.extend(cfg.predecessors(current));
                }
            }
        }

        initial_in = FixedBitSet::new();
    }
    solution
}

fn construct_postorder(cfg: &Cfg) -> Vec<BasicBlockIdx> {
    fn helper(
        cfg: &Cfg,
        current: BasicBlockIdx,
        visited: &mut SecondaryMap<BasicBlockIdx, bool>,
        traversal: &mut Vec<BasicBlockIdx>,
    ) {
        visited.insert(current, true);
        for successor in cfg.successors(current) {
            if !visited.contains_key(successor) {
                helper(cfg, successor, visited, traversal);
            }
        }
        traversal.push(current);
    }

    let mut traversal = vec![];
    let mut visited = SecondaryMap::with_capacity(cfg.vertices.capacity());
    helper(cfg, cfg.entry, &mut visited, &mut traversal);
    traversal
}
