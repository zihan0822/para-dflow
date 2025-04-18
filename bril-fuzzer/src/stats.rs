#![allow(dead_code)]
use bril_rs::program::Type;
pub const ALL_TYPES: [Type; 2] = [Type::Int, Type::Bool];
pub mod func {
    use super::*;
    pub const NUM_ARGS: [usize; 4] = [0, 1, 2, 3];
    pub const NUM_ARGS_W: [f64; 4] = [0.2, 0.4, 0.4, 0.2];
    pub const ARGS_TY: [Type; 2] = ALL_TYPES;
    pub const ARGS_TY_W: [f64; 2] = [0.8, 0.2];
}

pub mod instr {
    use super::*;
    pub const CONST_OR_ELSE_W: [f64; 2] = [0.4, 0.6];
    pub const INSTR_TY: [Type; 2] = ALL_TYPES;
    pub const INSTR_TY_W: [f64; 2] = [0.8, 0.2];
}
