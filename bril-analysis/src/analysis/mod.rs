mod reaching_def;
mod prelude {
    pub(crate) use super::InstructionExt;
    pub(crate) use crate::{Direction, solve_dataflow};
    pub(crate) use bril::builder::BasicBlockIdx;
    pub(crate) use bril_cfg::Cfg;
    pub(crate) use fixedbitset::FixedBitSet;
    pub(crate) use slotmap::SecondaryMap;
}
use bril::ir::{Instruction, Variable};

pub use reaching_def::reaching_def;

pub(crate) trait InstructionExt {
    fn dest(&self) -> Option<&Variable>;
}

impl InstructionExt for Instruction {
    fn dest(&self) -> Option<&Variable> {
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
            | Instruction::Id(dest, ..) => Some(dest),
            Instruction::Call(dest, ..) => dest.as_ref(),
            _ => None,
        }
    }
}
