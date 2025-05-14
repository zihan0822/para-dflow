// Copyright (C) 2025 Zihan Li and Ethan Uppal.

mod dist;
mod fuzzer;
mod instr;

use clap::Parser;
use fuzzer::FuzzerBuilder;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

#[derive(Parser, Debug)]
struct Args {
    /// Random seed
    #[arg(short, long)]
    seed: Option<u64>,

    /// Number of blocks in the emitted function
    #[arg(short = 'n', long, default_value_t = 1024)]
    num_blocks: usize,

    /// Block size mean (of normal distribution)
    #[arg(long, default_value_t = 32)]
    block_size_mean: usize,

    /// Block size std (of normal distribution)
    #[arg(long, default_value_t = 0.0)]
    block_size_std: f64,

    /// Maximum nesting level, including if-else and loop constructs
    #[arg(long = "max-nesting", default_value_t = 3)]
    max_block_depth: usize,
}

fn main() {
    let args = Args::parse();
    let mut rng = if let Some(seed) = args.seed {
        ChaCha8Rng::seed_from_u64(seed)
    } else {
        ChaCha8Rng::from_os_rng()
    };
    let builder = FuzzerBuilder::with_rng(&mut rng);
    let mut fuzzer = builder
        .num_blocks(args.num_blocks)
        .block_size(args.block_size_mean, args.block_size_std)
        .max_block_depth(args.max_block_depth)
        .finish();
    let prog = fuzzer.fuzz();

    let mut buf = String::new();
    let mut pretty_printer = bril::printer::Printer::new(&mut buf);
    pretty_printer.print_program(&prog).unwrap();
    println!("{:#}", buf);
}
