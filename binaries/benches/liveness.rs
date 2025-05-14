use divan::{Bencher, black_box};
use std::io::{BufReader, Read};

fn main() {
    divan::main();
}

fn prepare_bench_programs() -> Vec<bril::ir::Program> {
    let mut programs = vec![];
    for entry in std::fs::read_dir("inputs").unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file()
            || path.extension().and_then(|s| s.to_str()) != Some("json")
        {
            continue;
        }
        let mut reader = BufReader::new(std::fs::File::open(&path).unwrap());
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();

        let bril_prog: bril_rs::Program = serde_json::from_str(&buf).unwrap();
        programs.push(bril::shim::flattened_program_repr(bril_prog))
    }
    programs
}

#[divan::bench]
fn parallel_run(bencher: Bencher) {
    let programs = prepare_bench_programs();
    let cfgs: Vec<_> = programs
        .iter()
        .flat_map(|program| {
            program
                .functions()
                .map(|function| bril_cfg::build_cfg(&function))
        })
        .collect();
    bencher.bench_local(|| {
        for cfg in &cfgs {
            black_box(bril_analysis::analysis::liveness_para(cfg, 8));
        }
    })
}

#[divan::bench]
fn sequential_run(bencher: Bencher) {
    let programs = prepare_bench_programs();
    let cfgs: Vec<_> = programs
        .iter()
        .flat_map(|program| {
            program
                .functions()
                .map(|function| bril_cfg::build_cfg(&function))
        })
        .collect();
    bencher.bench_local(|| {
        for cfg in &cfgs {
            black_box(bril_analysis::analysis::liveness(cfg));
        }
    })
}
