use crate::dist::*;
use bril_rs::program::*;
use rand::Rng;
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<rand::rngs::ThreadRng> = RefCell::new(rand::rng());
}

pub fn generate_bril_program(num_fns: usize) -> Program {
    Program {
        functions: (0..num_fns)
            .map(|_| {
                let prototype = RNG.with_borrow_mut(|rng| rng.sample(BrilDist));
                generate_fn(8, prototype)
            })
            .collect(),
    }
}

fn generate_fn(num_instrs: usize, prototype: Prototype) -> Function {
    let instrs = generate_code_blk(num_instrs, Context::from_prototype(&prototype));
    Function {
        args: prototype.args,
        instrs,
        name: prototype.name,
        return_type: prototype.return_type,
    }
}

fn generate_code_blk(num_instrs: usize, mut ctx: Context) -> Vec<Code> {
    let mut instrs = vec![];
    for _ in 0..num_instrs {
        let next = RNG.with_borrow_mut(|rng| ArithInst::sample_with_ctx(&ctx, rng));
        let (dest, op_type) = parse_dest_and_ty(&next.0);
        ctx.insert_new_local_var(dest, op_type);
        instrs.push(Code::Instruction(next.0));
    }
    instrs
}

fn parse_dest_and_ty(instr: &Instruction) -> (String, Type) {
    match instr {
        Instruction::Value { dest, op_type, .. } => (dest.clone(), op_type.clone()),
        Instruction::Constant {
            dest, const_type, ..
        } => (dest.clone(), const_type.clone()),
        _ => unreachable!(),
    }
}
