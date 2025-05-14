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
    let prog = bril::shim::flattened_program_repr(bril_prog);

    for function in prog.functions() {
        let cfg = build_cfg(&function);
        let parallel_res: SecondaryMap<_, _> =
            bril_analysis::analysis::reaching_def_para(&cfg, 4)
                .into_iter()
                .map(|(k, v)| (k, v.clone()))
                .collect();
        let sequential_res = bril_analysis::analysis::reaching_def(&cfg);
        assert_eq!(parallel_res, sequential_res);
    }
    eprintln!("passed!");
    Ok(())
}
