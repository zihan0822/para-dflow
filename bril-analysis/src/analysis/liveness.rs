use super::prelude::*;
use std::collections::{HashMap, HashSet};

/// ones of the returned bitset should be interpreted as the numbering of live
/// variable variables are zero-indexed per function
pub fn liveness(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let (kill_set, gen_set) = (find_kill_set(cfg), find_gen_set(cfg));
    sequential::solve_dataflow(
        cfg,
        &(),
        Direction::Backward,
        HashMap::new(),
        |mut in1, in2| {
            in1.union_with(in2);
            in1
        },
        |block_idx, mut merged_in| {
            merged_in.difference_with(&kill_set[block_idx]);
            merged_in.union_with(&gen_set[block_idx]);
            merged_in
        },
    )
}

pub fn liveness_para(
    cfg: &Cfg,
    num_threads: usize,
) -> DashMap<BasicBlockIdx, FixedBitSet> {
    let (kill_set, gen_set) = (find_kill_set(cfg), find_gen_set(cfg));
    parallel::solve_dataflow(
        cfg,
        Direction::Backward,
        FixedBitSet::new(),
        |mut in1, in2| {
            in1.union_with(in2);
            in1
        },
        |block_idx, mut merged_in| {
            merged_in.difference_with(&kill_set[block_idx]);
            merged_in.union_with(&gen_set[block_idx]);
            merged_in
        },
        num_threads,
    )
}

fn find_kill_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let mut kill_set = SecondaryMap::new();
    for (idx, block) in cfg.vertices.iter() {
        let mut able_to_kill = FixedBitSet::new();
        for instruction in block.instructions.iter().rev() {
            if let Some(dest) = instruction.dest() {
                able_to_kill.grow_and_insert(dest.0 as usize);
            }
        }
        kill_set.insert(idx, able_to_kill);
    }
    kill_set
}

fn find_gen_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let mut gen_set = SecondaryMap::new();
    for (idx, block) in cfg.vertices.iter() {
        let mut generated = FixedBitSet::new();
        let mut local_defs = HashSet::new();
        for instruction in block.instructions {
            for operand in instruction.operands() {
                if !local_defs.contains(&operand.0) {
                    generated.grow_and_insert(operand.0 as usize);
                }
            }
            if let Some(dest) = instruction.dest() {
                local_defs.insert(dest.0);
            }
        }
        gen_set.insert(idx, generated);
    }
    gen_set
}
