use bril_rs::program::*;
use rand::seq::IndexedRandom;
use rand::{
    Rng,
    distr::{self, Alphanumeric, Distribution},
};
use std::collections::HashMap;

pub trait Sample
where
    BrilDist: Distribution<Self>,
    Self: Sized,
{
    type Context;
    fn sample_with_ctx<R: Rng + ?Sized>(ctx: &Self::Context, rng: &mut R) -> Self;
    fn sample<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.sample(BrilDist)
    }
}

pub mod stats {
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
}

pub struct BrilDist;

pub struct Prototype {
    pub name: String,
    pub args: Vec<Argument>,
    pub return_type: Option<Type>,
}

impl Distribution<Prototype> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Prototype {
        let num_args =
            *sample_one_by_weights(&stats::func::NUM_ARGS, &stats::func::NUM_ARGS_W, rng);
        let name = generate_random_ident(rng);
        let args: Vec<_> = (0..num_args)
            .map(|_| {
                let arg_type =
                    sample_one_by_weights(&stats::func::ARGS_TY, &stats::func::ARGS_TY_W, rng)
                        .clone();
                Argument {
                    name: generate_random_ident(rng),
                    arg_type,
                }
            })
            .collect();
        Prototype {
            name,
            args,
            return_type: None,
        }
    }
}

#[derive(Default)]
pub struct Context {
    pub local_vars: HashMap<Type, Vec<String>>,
    labels: Vec<String>,
    fns: Vec<Prototype>,
}

impl Context {
    pub fn from_prototype(prototype: &Prototype) -> Self {
        let mut local_vars: HashMap<_, Vec<String>> = HashMap::new();
        for arg in &prototype.args {
            local_vars
                .entry(arg.arg_type.clone())
                .or_default()
                .push(arg.name.clone());
        }
        Self {
            local_vars,
            labels: vec![],
            fns: vec![],
        }
    }

    /// sample with replacement
    pub fn sample_operands_of_ty<R: Rng + ?Sized>(
        &self,
        ty: Type,
        num: usize,
        rng: &mut R,
    ) -> Option<Vec<String>> {
        self.local_vars.get(&ty).and_then(|candidates| {
            if candidates.is_empty() {
                None
            } else {
                Some(
                    (0..num)
                        .map(|_| candidates.choose(rng).unwrap())
                        .cloned()
                        .collect(),
                )
            }
        })
    }

    pub fn insert_new_local_var(&mut self, var: String, ty: Type) {
        self.local_vars
            .entry(ty)
            .and_modify(|vars| {
                vars.push(var.clone());
                vars.dedup()
            })
            .or_insert(vec![var]);
    }

    pub fn intersect(&mut self, other: Self) {
        todo!()
    }
}

#[derive(Clone)]
pub struct ArithInst(pub Instruction);

#[derive(Clone)]
pub struct BoolInst(pub Instruction);

impl Distribution<ArithInst> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ArithInst {
        ArithInst(Instruction::Constant {
            dest: generate_random_ident(rng),
            op: ConstOps::Const,
            const_type: Type::Int,
            value: Literal::Int(rng.random::<i8>() as _),
        })
    }
}

impl Sample for ArithInst {
    type Context = Context;
    fn sample_with_ctx<R: Rng + ?Sized>(ctx: &Context, rng: &mut R) -> Self {
        let op = *sample_one_by_weights(
            &[
                ValueOps::Add,
                ValueOps::Sub,
                ValueOps::Mul,
                ValueOps::Div,
                ValueOps::Id,
            ],
            &[1.0; 5],
            rng,
        );
        let num_args = if matches!(op, ValueOps::Id) { 1 } else { 2 };
        if let Some(args) = ctx.sample_operands_of_ty(Type::Int, num_args, rng) {
            ArithInst(Instruction::Value {
                args,
                dest: generate_random_ident(rng),
                funcs: vec![],
                labels: vec![],
                op,
                op_type: Type::Int,
            })
        } else {
            // fallback to direct sample
            <ArithInst as Sample>::sample(rng)
        }
    }
}

impl Distribution<BoolInst> for BrilDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BoolInst {
        BoolInst(Instruction::Constant {
            dest: generate_random_ident(rng),
            op: ConstOps::Const,
            const_type: Type::Bool,
            value: Literal::Bool(rng.random()),
        })
    }
}

impl Sample for BoolInst {
    type Context = Context;
    fn sample_with_ctx<R: Rng + ?Sized>(ctx: &Context, rng: &mut R) -> Self {
        let op = *sample_one_by_weights(
            &[
                ValueOps::Lt,
                ValueOps::Gt,
                ValueOps::Le,
                ValueOps::Ge,
                ValueOps::Not,
                ValueOps::And,
                ValueOps::Or,
                ValueOps::Eq,
            ],
            &[1.0; 8],
            rng,
        );
        let num_args = if matches!(op, ValueOps::Not) { 1 } else { 2 };
        if let Some(args) = ctx.sample_operands_of_ty(Type::Bool, num_args, rng) {
            BoolInst(Instruction::Value {
                args,
                dest: generate_random_ident(rng),
                funcs: vec![],
                labels: vec![],
                op,
                op_type: Type::Bool,
            })
        } else {
            <Self as Sample>::sample(rng)
        }
    }
}

fn generate_random_ident<R: Rng + ?Sized>(rng: &mut R) -> String {
    const MAX_IDENT_LEN: usize = 8;
    let mut len = rng.random_range(1..MAX_IDENT_LEN);
    let first_char = std::iter::once('_')
        .chain('a'..='z')
        .chain('A'..='Z')
        .collect::<Vec<_>>()
        .choose(rng)
        .copied()
        .unwrap();

    if first_char == '_' && len == 1 {
        len += 1;
    }
    let rest: String = rng
        .sample_iter(Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    format!("{first_char}{rest}")
}

/// sample one element from input slice according to a weight vector
fn sample_one_by_weights<'a, T, R: Rng + ?Sized>(
    s: &'a [T],
    weights: &[f64],
    rng: &mut R,
) -> &'a T {
    let weighted_vec = distr::weighted::WeightedIndex::new(weights).unwrap();
    &s[weighted_vec.sample(rng)]
}
