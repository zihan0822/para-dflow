use super::prelude::*;
use std::collections::HashMap;

/// ones in the returned bitset should be interpreted as offset of the
/// instruction that defines the reaching definition relative to function's
/// instruction buffer
pub fn reaching_def(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let (kill_set, gen_set) = (find_kill_set(cfg), find_gen_set(cfg));
    // function parameters are not tracked
    sequential::solve_dataflow(
        cfg,
        &(),
        Direction::Forward,
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

pub fn reaching_def_para(
    cfg: &Cfg,
    num_threads: usize,
) -> DashMap<BasicBlockIdx, BitSet> {
    let (kill_set, gen_set) = (find_kill_set_para(cfg), find_gen_set_para(cfg));
    // function parameters are not tracked
    parallel::solve_dataflow(
        cfg,
        Direction::Forward,
        BitSet::new(),
        |mut in1, in2| {
            in1 |= &in2;
            in1
        },
        |block_idx, mut merged_in| {
            merged_in &=
                &(!kill_set.get(&block_idx).map(|kill| kill.clone()).unwrap());
            let generated: &BitSet = &gen_set.get(&block_idx).unwrap();
            merged_in |= generated;
            merged_in
        },
        num_threads,
    )
}

fn find_kill_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0) as u32;

    let mut universe: HashMap<u32, BitSet> = HashMap::new();
    for block in cfg.vertices.values() {
        for (i, instruction) in block.instructions.iter().enumerate() {
            if let Some(dest) = instruction.dest() {
                universe
                    .entry(dest.0)
                    .or_insert(BitSet::with_capacity(total_instr_num))
                    .add((block.offset + i) as u32);
            }
        }
    }

    let mut kill_set = SecondaryMap::with_capacity(cfg.vertices.capacity());
    for (idx, block) in cfg.vertices.iter() {
        let mut able_to_kill = block.instructions.iter().fold(
            BitSet::with_capacity(total_instr_num),
            |mut acc, instruction| {
                if let Some(dest) = instruction.dest() {
                    acc |= &universe[&dest.0];
                }
                acc
            },
        );
        for offset in block.offset..(block.offset + block.instructions.len()) {
            able_to_kill.remove(offset as u32);
        }
        kill_set.insert(idx, able_to_kill);
    }
    kill_set
}

fn find_gen_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, BitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0) as u32;

    let mut gen_set = SecondaryMap::with_capacity(cfg.vertices.capacity());
    for (idx, block) in cfg.vertices.iter() {
        let mut generated: HashMap<u32, usize> = HashMap::new();
        for (i, instruction) in block.instructions.iter().enumerate().rev() {
            if let Some(dest) = instruction.dest() {
                generated.entry(dest.0).or_insert(block.offset + i);
            }
        }
        gen_set.insert(
            idx,
            generated.into_values().fold(
                BitSet::with_capacity(total_instr_num),
                |mut acc, offset| {
                    acc.add(offset as u32);
                    acc
                },
            ),
        );
    }
    gen_set
}

fn find_kill_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, BitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0) as u32;

    let universe = cfg
        .vertices
        .values()
        .par_bridge()
        .map(|block| {
            let mut partial_universe = HashMap::new();
            for (i, instruction) in block.instructions.iter().enumerate() {
                if let Some(dest) = instruction.dest() {
                    partial_universe
                        .entry(dest.0)
                        .or_insert(BitSet::with_capacity(total_instr_num))
                        .add((block.offset + i) as u32);
                }
            }
            partial_universe
        })
        .reduce(HashMap::new, |mut u1, u2| {
            for (variable, defs2) in u2 {
                u1.entry(variable)
                    .and_modify(|defs1| *defs1 |= &defs2)
                    .or_insert(defs2);
            }
            u1
        });

    let kill_set = DashMap::new();
    cfg.vertices.iter().par_bridge().for_each(|(idx, block)| {
        let mut able_to_kill = block.instructions.iter().fold(
            BitSet::with_capacity(total_instr_num),
            |mut acc, instruction| {
                if let Some(dest) = instruction.dest() {
                    acc |= &universe[&dest.0];
                }
                acc
            },
        );
        for offset in block.offset..(block.offset + block.instructions.len()) {
            able_to_kill.remove(offset as u32);
        }
        kill_set.insert(idx, able_to_kill);
    });
    kill_set
}

fn find_gen_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, BitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0) as u32;

    let gen_set = DashMap::new();
    cfg.vertices.iter().par_bridge().for_each(|(idx, block)| {
        let mut generated: HashMap<u32, usize> = HashMap::new();
        for (i, instruction) in block.instructions.iter().enumerate().rev() {
            if let Some(dest) = instruction.dest() {
                generated.entry(dest.0).or_insert(block.offset + i);
            }
        }
        gen_set.insert(
            idx,
            generated.into_values().fold(
                BitSet::with_capacity(total_instr_num),
                |mut acc, offset| {
                    acc.add(offset as u32);
                    acc
                },
            ),
        );
    });
    gen_set
}
