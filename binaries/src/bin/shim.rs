use bril::{printer, shim};
use bril_rs::Program;
use clap::Parser;
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

    let mut buf = String::new();
    let mut pretty_printer = printer::Printer::new(&mut buf);
    pretty_printer.print_program(&prog).unwrap();
    println!("{:#}", buf);

    Ok(())
}
