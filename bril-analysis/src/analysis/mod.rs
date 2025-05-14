mod liveness;
mod reaching_def;
mod prelude {
    pub(crate) use super::InstructionExt;
    pub(crate) use crate::{Direction, parallel, sequential};
    pub(crate) use bril::builder::BasicBlockIdx;
    pub(crate) use bril_cfg::Cfg;
    pub(crate) use dashmap::DashMap;
    pub(crate) use hibitset::BitSet;
    pub(crate) use rayon::prelude::*;
    pub(crate) use slotmap::SecondaryMap;
}
use bril::ir::{Instruction, Variable};

pub use liveness::*;
pub use reaching_def::*;

pub(crate) trait InstructionExt {
    fn dest(&self) -> Option<Variable>;
    fn operands(&self) -> Vec<Variable>;
}

impl InstructionExt for Instruction {
    fn dest(&self) -> Option<Variable> {
        match self {
            Instruction::Add(dest, ..)
            | Instruction::Sub(dest, ..)
            | Instruction::Mul(dest, ..)
            | Instruction::Div(dest, ..)
            | Instruction::Eq(dest, ..)
            | Instruction::Lt(dest, ..)
            | Instruction::Gt(dest, ..)
            | Instruction::Le(dest, ..)
            | Instruction::Ge(dest, ..)
            | Instruction::Not(dest, ..)
            | Instruction::And(dest, ..)
            | Instruction::Or(dest, ..)
            | Instruction::Const(dest, ..)
            | Instruction::Id(dest, ..) => Some(*dest),
            Instruction::Call(dest, ..) => dest.as_ref().copied(),
            _ => None,
        }
    }

    fn operands(&self) -> Vec<Variable> {
        match self {
            Instruction::Add(_, arg0, arg1)
            | Instruction::Sub(_, arg0, arg1)
            | Instruction::Mul(_, arg0, arg1)
            | Instruction::Div(_, arg0, arg1)
            | Instruction::Eq(_, arg0, arg1)
            | Instruction::Lt(_, arg0, arg1)
            | Instruction::Gt(_, arg0, arg1)
            | Instruction::Le(_, arg0, arg1)
            | Instruction::Ge(_, arg0, arg1)
            | Instruction::And(_, arg0, arg1)
            | Instruction::Or(_, arg0, arg1) => vec![*arg0, *arg1],
            Instruction::Not(.., arg0) | Instruction::Id(.., arg0) => {
                vec![*arg0]
            }
            Instruction::Call(.., args) | Instruction::Print(args) => {
                args.to_vec()
            }
            Instruction::Br(arg0, ..) => vec![*arg0],
            Instruction::Ret(ret) => ret.iter().copied().collect(),
            _ => vec![],
        }
    }
}
