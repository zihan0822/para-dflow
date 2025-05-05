// Copyright (C) 2025 Zihan Li and Ethan Uppal.

mod dist;
mod fuzzer;
mod instr;

use fuzzer::FuzzerBuilder;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let builder = FuzzerBuilder::with_rng(&mut rng);
    let mut fuzzer = builder
        .num_blocks(8)
        .block_size(8, 2.0)
        .max_block_depth(5)
        .finish();
    let prog = fuzzer.fuzz();

    let mut buf = String::new();
    let mut pretty_printer = bril::printer::Printer::new(&mut buf);
    pretty_printer.print_program(&prog).unwrap();
    println!("{:#}", buf);
}
