use bril::shim;
use bril_cfg::build_cfg;
use bril_rs::Program;
use clap::Parser;
use slotmap::SecondaryMap;
use std::io::{BufReader, Read};

#[derive(Parser)]
struct Args {
    #[arg(short)]
    f: Option<String>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut reader: Box<dyn Read> = if let Some(ref f) = args.f {
        Box::new(BufReader::new(std::fs::File::open(f)?))
    } else {
        Box::new(BufReader::new(std::io::stdin()))
    };
    let mut buf = String::new();
    assert!(reader.read_to_string(&mut buf)? > 0);

    let bril_prog: Program = serde_json::from_str(&buf).unwrap();
    let prog = shim::flattened_program_repr(bril_prog);

    for function in prog.functions() {
        let cfg = build_cfg(&function);
        let mut para_result = SecondaryMap::new();
        let res = bril_analysis::analysis::liveness_para(&cfg, 4);
        // println!("parallel");
        for entry in res.iter() {
            // println!(".{}", cfg.vertices[*entry.key()].label.unwrap().name);
            // for variable in entry.value().ones() {
            //     println!("x{}", variable);
            // }
            para_result.insert(*entry.key(), entry.value().clone());
        }

        // println!("sequential");
        let sequential_res = bril_analysis::analysis::liveness(&cfg);
        // for (k, v) in sequential_res.iter() {
        //     println!(".{}", cfg.vertices[k].label.unwrap().name);
        //     for variable in v.ones() {
        //         println!("x{}", variable);
        //     }
        // }
        for (k, v) in para_result.iter() {
            if !v.eq(&sequential_res[k]) {
                panic!("{:#?} != {:#?}", v, sequential_res[k]);
            }
        }
    }
    eprintln!("passed!");

    Ok(())
}
