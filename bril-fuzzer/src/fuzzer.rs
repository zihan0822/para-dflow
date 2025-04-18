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
    #[derive(Sample)]
    enum BoolOrArith {
        #[w = 0.2]
        Bool(BoolInst),
        #[w = 0.8]
        Arith(ArithInst),
    }

    for _ in 0..num_instrs {
        let next = match RNG.with_borrow_mut(|rng| BoolOrArith::sample_with_ctx(&ctx, rng)) {
            BoolOrArith::Bool(bool_instr) => bool_instr.0,
            BoolOrArith::Arith(arith_instr) => arith_instr.0,
        };
        let (dest, op_type) = parse_dest_and_ty(&next);
        ctx.insert_new_local_var(dest, op_type);
        instrs.push(Code::Instruction(next));
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
