mod dist;
mod fuzzer;

fn main() {
    let bril_program = serde_json::to_string(&fuzzer::generate_bril_program(1)).unwrap();
    println!("{}", bril_program);
}
