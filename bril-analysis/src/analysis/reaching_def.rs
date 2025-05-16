use fixedbitset::SimdBlock;

use super::prelude::*;
use std::{
    collections::HashMap,
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

/// ones in the returned bitset should be interpreted as offset of the
/// instruction that defines the reaching definition relative to function's
/// instruction buffer
pub fn reaching_def(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let (kill_set, gen_set) = (find_kill_set(cfg), find_gen_set(cfg));
    // function parameters are not tracked
    sequential::solve_dataflow(
        cfg,
        &(),
        Direction::Forward,
        HashMap::new(),
        |mut in1, in2| {
            in1.grow(in2.len());
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

pub fn reaching_def_para(
    cfg: &Cfg,
    num_threads: usize,
) -> DashMap<BasicBlockIdx, FixedBitSet> {
    let (kill_set, gen_set) = (find_kill_set_para(cfg), find_gen_set_para(cfg));
    // function parameters are not tracked
    parallel::solve_dataflow(
        cfg,
        Direction::Forward,
        FixedBitSet::new(),
        |mut in1, in2| {
            in1.union_with(in2);
            in1
        },
        |block_idx, mut merged_in| {
            merged_in.difference_with(&kill_set.get(&block_idx).unwrap());
            merged_in.union_with(&gen_set.get(&block_idx).unwrap());
            merged_in
        },
        num_threads,
    )
}

fn find_kill_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0);

    let mut universe: HashMap<u32, FixedBitSet> = HashMap::new();
    for block in cfg.vertices.values() {
        for (i, instruction) in block.instructions.iter().enumerate() {
            if let Some(dest) = instruction.dest() {
                universe
                    .entry(dest.0)
                    .or_insert(FixedBitSet::with_capacity(total_instr_num))
                    .insert(block.offset + i);
            }
        }
    }

    let mut kill_set = SecondaryMap::with_capacity(cfg.vertices.capacity());
    for (idx, block) in cfg.vertices.iter() {
        let mut able_to_kill = block.instructions.iter().fold(
            FixedBitSet::with_capacity(total_instr_num),
            |mut acc, instruction| {
                if let Some(dest) = instruction.dest() {
                    acc.union_with(&universe[&dest.0]);
                }
                acc
            },
        );
        able_to_kill.remove_range(
            block.offset..(block.offset + block.instructions.len()),
        );
        kill_set.insert(idx, able_to_kill);
    }
    kill_set
}

fn find_gen_set(cfg: &Cfg) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0);

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
                FixedBitSet::with_capacity(total_instr_num),
                |mut acc, offset| {
                    acc.insert(offset);
                    acc
                },
            ),
        );
    }
    gen_set
}

fn find_kill_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, FixedBitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0);
    let mut universe: HashMap<u32, FixedBitSet> = HashMap::new();

    let (mut block_count, rem) = (
        total_instr_num / SimdBlock::BITS,
        total_instr_num % SimdBlock::BITS,
    );
    block_count += (rem > 0) as usize;
    let arena_size = block_count * (total_instr_num + cfg.vertices.len());
    let arena: Box<[SimdBlock]> =
        vec![SimdBlock::NONE; arena_size].into_boxed_slice();

    for block in cfg.vertices.values() {
        for (i, instruction) in block.instructions.iter().enumerate() {
            if let Some(dest) = instruction.dest() {
                universe
                    .entry(dest.0)
                    .or_insert(unsafe {
                        let segment = arena
                            .as_ptr()
                            .add(block_count * (block.offset + i))
                            as *mut MaybeUninit<SimdBlock>;
                        std::ptr::write_bytes(segment, 0, block_count);
                        let start = NonNull::new_unchecked(segment);
                        let a = FixedBitSet::stupid(total_instr_num, start);
                        assert!(a.is_clear(), "{a}");
                        a
                    })
                    .insert(block.offset + i);
            }
        }
    }

    let kill_set = DashMap::new();
    cfg.vertices.iter().enumerate().par_bridge().for_each(
        |(id, (idx, block))| {
            let mut able_to_kill = block.instructions.iter().fold(
                unsafe {
                    let segment = arena
                        .as_ptr()
                        .add(block_count * (total_instr_num + id))
                        as *mut MaybeUninit<SimdBlock>;
                    std::ptr::write_bytes(segment, 0, block_count);
                    let start = NonNull::new_unchecked(segment);
                    let a = FixedBitSet::stupid(total_instr_num, start);
                    assert!(a.is_clear());
                    a
                },
                |mut acc, instruction| {
                    if let Some(dest) = instruction.dest() {
                        acc.union_with(&universe[&dest.0]);
                    }
                    acc
                },
            );
            able_to_kill.remove_range(
                block.offset..(block.offset + block.instructions.len()),
            );
            kill_set.insert(idx, able_to_kill);
        },
    );

    mem::forget(arena);

    kill_set
}

fn find_gen_set_para(cfg: &Cfg) -> DashMap<BasicBlockIdx, FixedBitSet> {
    let total_instr_num = cfg
        .vertices
        .values()
        .map(|v| v.offset + v.instructions.len())
        .max()
        .unwrap_or(0);

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
                FixedBitSet::with_capacity(total_instr_num),
                |mut acc, offset| {
                    acc.insert(offset);
                    acc
                },
            ),
        );
    });
    gen_set
}
