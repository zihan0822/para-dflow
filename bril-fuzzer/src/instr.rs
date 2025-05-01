use bril::ir::{Instruction, Type, Value, Variable};

pub trait InstrConfig {
    fn num_operands(&self) -> usize;
    fn dest_ty(&self) -> Type;
    fn operands_ty(&self) -> Type;
    fn config_operands(&mut self, operands: impl IntoIterator<Item = Variable>);
    fn config_dest(&mut self, dest: Variable);
}

impl InstrConfig for Instruction {
    fn num_operands(&self) -> usize {
        match self {
            Instruction::Not(..) => 1,
            Instruction::Add(..)
            | Instruction::Sub(..)
            | Instruction::Mul(..)
            | Instruction::Div(..)
            | Instruction::Le(..)
            | Instruction::Ge(..)
            | Instruction::Lt(..)
            | Instruction::Gt(..)
            | Instruction::And(..)
            | Instruction::Or(..)
            | Instruction::Eq(..) => 2,
            _ => todo!(),
        }
    }

    fn dest_ty(&self) -> Type {
        match self {
            Instruction::Add(..)
            | Instruction::Sub(..)
            | Instruction::Mul(..)
            | Instruction::Div(..) => Type::Int,

            Instruction::Le(..)
            | Instruction::Ge(..)
            | Instruction::Lt(..)
            | Instruction::Gt(..)
            | Instruction::And(..)
            | Instruction::Or(..)
            | Instruction::Eq(..)
            | Instruction::Not(..) => Type::Bool,

            Instruction::Const(_, val) => match val {
                Value::Int(_) => Type::Int,
                Value::Bool(_) => Type::Bool,
            },
            _ => todo!(),
        }
    }

    fn operands_ty(&self) -> Type {
        match self {
            Instruction::Add(..)
            | Instruction::Sub(..)
            | Instruction::Mul(..)
            | Instruction::Div(..)
            | Instruction::Le(..)
            | Instruction::Ge(..)
            | Instruction::Lt(..)
            | Instruction::Gt(..)
            | Instruction::Eq(..) => Type::Int,
            Instruction::And(..)
            | Instruction::Or(..)
            | Instruction::Not(..) => Type::Bool,
            _ => todo!(),
        }
    }

    fn config_dest(&mut self, var: Variable) {
        match self {
            Instruction::Add(dest, ..)
            | Instruction::Sub(dest, ..)
            | Instruction::Mul(dest, ..)
            | Instruction::Div(dest, ..)
            | Instruction::Le(dest, ..)
            | Instruction::Ge(dest, ..)
            | Instruction::Lt(dest, ..)
            | Instruction::Gt(dest, ..)
            | Instruction::Eq(dest, ..)
            | Instruction::And(dest, ..)
            | Instruction::Or(dest, ..)
            | Instruction::Not(dest, ..)
            | Instruction::Const(dest, ..) => *dest = var,
            _ => todo!(),
        }
    }

    fn config_operands(
        &mut self,
        operands: impl IntoIterator<Item = Variable>,
    ) {
        let mut operands = operands.into_iter();
        match self {
            Instruction::Add(_, arg0, arg1)
            | Instruction::Sub(_, arg0, arg1)
            | Instruction::Mul(_, arg0, arg1)
            | Instruction::Div(_, arg0, arg1)
            | Instruction::Le(_, arg0, arg1)
            | Instruction::Ge(_, arg0, arg1)
            | Instruction::Lt(_, arg0, arg1)
            | Instruction::Gt(_, arg0, arg1)
            | Instruction::Eq(_, arg0, arg1)
            | Instruction::And(_, arg0, arg1)
            | Instruction::Or(_, arg0, arg1) => {
                *arg0 = operands.next().unwrap();
                *arg1 = operands.next().unwrap();
            }
            Instruction::Not(_, arg0) => *arg0 = operands.next().unwrap(),
            _ => {
                todo!();
            }
        }
    }
}
