use super::prelude::*;
use std::collections::{HashMap, HashSet};

/// ones of the returned bitset should be interpreted as the numbering of live
/// variable variables are zero-indexed per function
pub fn liveness(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let (kill_set, gen_set) = (find_kill_set(cfg), find_gen_set(cfg));
    sequential::solve_dataflow(
        cfg,
        &(),
        Direction::Backward,
        HashMap::new(),
        |mut in1, in2| {
            in1 |= &in2;
            in1
        },
        |block_idx, mut merged_in| {
            merged_in &= &(!kill_set[block_idx].clone());
            merged_in |= &gen_set[block_idx];
            merged_in
        },
    )
}

pub fn liveness_para(
    cfg: &Cfg,
    num_threads: usize,
) -> DashMap<BasicBlockIdx, BitSet> {
    let (kill_set, gen_set) = (find_kill_set_para(cfg), find_gen_set_para(cfg));
    parallel::solve_dataflow(
        cfg,
        Direction::Backward,
        BitSet::new(),
        |mut in1, in2| {
            in1 |= &in2;
            in1
        },
        |block_idx, mut merged_in| {
            merged_in &= &(!kill_set.get(&block_idx).map(|kill| kill.clone()).unwrap());
            let generated: &BitSet = &gen_set.get(&block_idx).unwrap();
            merged_in |= generated;
            merged_in
        },
        num_threads,
    )
}

fn find_kill_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let mut kill_set = SecondaryMap::new();
    for (idx, block) in cfg.vertices.iter() {
        let mut able_to_kill = BitSet::new();
        for instruction in block.instructions.iter().rev() {
            if let Some(dest) = instruction.dest() {
                able_to_kill.add(dest.0 as u32);
            }
        }
        kill_set.insert(idx, able_to_kill);
    }
    kill_set
}

fn find_gen_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let mut gen_set = SecondaryMap::new();
    for (idx, block) in cfg.vertices.iter() {
        let mut generated = BitSet::new();
        let mut local_defs = HashSet::new();
        for instruction in block.instructions {
            for operand in instruction.operands() {
                if !local_defs.contains(&operand.0) {
                    generated.add(operand.0 as u32);
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

fn find_kill_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, BitSet> {
    let kill_set = DashMap::new();
    cfg.vertices.iter().par_bridge().for_each(|(idx, block)| {
        let mut able_to_kill = BitSet::new();
        for instruction in block.instructions.iter().rev() {
            if let Some(dest) = instruction.dest() {
                able_to_kill.add(dest.0 as u32);
            }
        }
        kill_set.insert(idx, able_to_kill);
    });
    kill_set
}

fn find_gen_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, BitSet> {
    let gen_set = DashMap::new();
    cfg.vertices.iter().par_bridge().for_each(|(idx, block)| {
        let mut generated = BitSet::new();
        let mut local_defs = HashSet::new();
        for instruction in block.instructions {
            for operand in instruction.operands() {
                if !local_defs.contains(&operand.0) {
                    generated.add(operand.0 as u32);
                }
            }
            if let Some(dest) = instruction.dest() {
                local_defs.insert(dest.0);
            }
        }
        gen_set.insert(idx, generated);
    });
    gen_set
}
