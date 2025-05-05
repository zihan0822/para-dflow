// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use bril::ir::{Instruction, Type, Value, Variable};
use rand::{
    distr::{self, Distribution},
    Rng,
};

pub struct BrilDist;

pub struct FuzzedValueInstr(pub Instruction);
pub struct FuzzedConstInstr(pub Instruction);
pub struct FuzzedType(pub Type);

const UNDEF_INT: Variable = Variable(u32::MAX, Type::Int);
const UNDEF_BOOL: Variable = Variable(u32::MAX, Type::Bool);

impl Distribution<FuzzedType> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FuzzedType {
        FuzzedType(*sample_one_by_weights(
            &[Type::Int, Type::Bool],
            &[0.75, 0.25],
            rng,
        ))
    }
}

impl Distribution<FuzzedConstInstr> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FuzzedConstInstr {
        let ty: FuzzedType = BrilDist.sample(rng);
        let const_instr = match ty.0 {
            Type::Bool => {
                Instruction::Const(UNDEF_BOOL, Value::Bool(rng.random()))
            }
            Type::Int => Instruction::Const(
                UNDEF_INT,
                Value::Int(rng.random::<i8>() as i64),
            ),
        };
        FuzzedConstInstr(const_instr)
    }
}

enum ValueOps {
    Add,
    Sub,
    Div,
    Mul,
    Lt,
    Gt,
    Le,
    Ge,
    Not,
    And,
    Or,
    Eq,
}

impl Distribution<FuzzedValueInstr> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FuzzedValueInstr {
        let operands_ty: FuzzedType = BrilDist.sample(rng);
        let value_instr = match operands_ty.0 {
            Type::Int => match sample_one_by_weights(
                &[ValueOps::Add, ValueOps::Sub, ValueOps::Div, ValueOps::Mul],
                &[1.0; 4],
                rng,
            ) {
                ValueOps::Add => {
                    Instruction::Add(UNDEF_INT, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Sub => {
                    Instruction::Sub(UNDEF_INT, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Mul => {
                    Instruction::Mul(UNDEF_INT, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Div => {
                    Instruction::Div(UNDEF_INT, UNDEF_INT, UNDEF_INT)
                }
                _ => unreachable!(),
            },
            Type::Bool => match sample_one_by_weights(
                &[
                    ValueOps::Lt,
                    ValueOps::Gt,
                    ValueOps::Le,
                    ValueOps::Ge,
                    ValueOps::Eq,
                    ValueOps::And,
                    ValueOps::Or,
                    ValueOps::Not,
                ],
                &[1.0; 8],
                rng,
            ) {
                ValueOps::Lt => {
                    Instruction::Lt(UNDEF_BOOL, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Gt => {
                    Instruction::Gt(UNDEF_BOOL, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Le => {
                    Instruction::Le(UNDEF_BOOL, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Ge => {
                    Instruction::Ge(UNDEF_BOOL, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::Eq => {
                    Instruction::Eq(UNDEF_BOOL, UNDEF_INT, UNDEF_INT)
                }
                ValueOps::And => {
                    Instruction::And(UNDEF_BOOL, UNDEF_BOOL, UNDEF_BOOL)
                }
                ValueOps::Or => {
                    Instruction::Or(UNDEF_BOOL, UNDEF_BOOL, UNDEF_BOOL)
                }
                ValueOps::Not => Instruction::Not(UNDEF_BOOL, UNDEF_BOOL),
                _ => unreachable!(),
            },
        };
        FuzzedValueInstr(value_instr)
    }
}

/// sample one element from input slice according to a weight vector
pub fn sample_one_by_weights<'a, T, R: Rng + ?Sized>(
    s: &'a [T],
    weights: &[f64],
    rng: &mut R,
) -> &'a T {
    let weighted_vec = distr::weighted::WeightedIndex::new(weights).unwrap();
    &s[weighted_vec.sample(rng)]
}
