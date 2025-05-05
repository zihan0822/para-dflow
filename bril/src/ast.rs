// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use slotmap::new_key_type;

use crate::ir::{Instruction, Variable};

new_key_type! { pub struct AstIdx; }

pub enum Ast {
    Instruction(Instruction),
    If(Variable, AstIdx, AstIdx),
    Seq(Vec<AstIdx>),
    Loop(Variable, AstIdx),
}
