use crate::{dist::*, instr::InstrConfig};

use bril::{
    ast::{Ast, AstIdx},
    ast_to_ir::ast_to_ir,
    ir::{Instruction, Program, Type, Variable},
};
use rand::prelude::*;
use slotmap::SlotMap;
use std::collections::HashMap;

pub struct Fuzzer<'a, R: Rng + ?Sized> {
    ctx: Context<'a, R>,
    root_ast_layout: AstLayout,
}

impl<R: Rng + ?Sized> Fuzzer<'_, R> {
    pub fn fuzz(&mut self) -> Program {
        let root = self.ctx.sample_ast(&self.root_ast_layout);
        ast_to_ir(&self.ctx.ast, root)
    }
}

pub struct FuzzerBuilder<'a, R: Rng + ?Sized> {
    config: FuzzConfig,
    num_blocks: usize,
    max_block_depth: usize,
    rng: &'a mut R,
}

impl<'a, R: Rng + ?Sized> FuzzerBuilder<'a, R> {
    pub fn with_rng(rng: &'a mut R) -> Self {
        Self {
            config: FuzzConfig {
                block_size_distr: rand_distr::Normal::new(20.0, 5.0).unwrap(),
            },
            num_blocks: 4,
            max_block_depth: 1,
            rng,
        }
    }

    pub fn block_size(mut self, mean: usize, std_dev: f64) -> Self {
        self.config.block_size_distr =
            rand_distr::Normal::new(mean as _, std_dev).unwrap();
        self
    }

    pub fn num_blocks(mut self, num_blocks: usize) -> Self {
        self.num_blocks = num_blocks;
        self
    }

    pub fn max_block_depth(mut self, level: usize) -> Self {
        self.max_block_depth = level;
        self
    }

    pub fn finish(self) -> Fuzzer<'a, R> {
        Fuzzer {
            ctx: Context {
                rng: self.rng,
                live_vars: HashMap::default(),
                ast: SlotMap::with_key(),
                next_var: 0,
                config: self.config,
            },
            root_ast_layout: AstLayout {
                num_blocks: self.num_blocks,
                max_block_depth: self.max_block_depth,
            },
        }
    }
}

struct FuzzConfig {
    block_size_distr: rand_distr::Normal<f64>,
}

struct SnapShot {
    next_var: usize,
    live_vars: HashMap<Type, Vec<Variable>>,
}

impl SnapShot {
    fn merge(&mut self, other: Self) {
        self.next_var = other.next_var.max(self.next_var);
        for (ty, vars) in &mut self.live_vars {
            vars.retain(|var| {
                other.live_vars.get(ty).is_some_and(|other_vars| {
                    other_vars.iter().any(|other_var| other_var.eq(var))
                })
            });
        }
    }
}

struct AstLayout {
    max_block_depth: usize,
    num_blocks: usize,
}

struct Context<'a, R: Rng + ?Sized> {
    rng: &'a mut R,
    live_vars: HashMap<Type, Vec<Variable>>,
    ast: SlotMap<AstIdx, Ast>,
    next_var: usize,
    config: FuzzConfig,
}

impl<R: Rng + ?Sized> Context<'_, R> {
    /// this might return None, if the required operands are not be sampled
    /// from live variables
    pub fn sample_instr(&mut self) -> Option<AstIdx> {
        enum ConstOrValue {
            Const,
            Value,
        }
        let mut instr = match sample_one_by_weights(
            &[ConstOrValue::Const, ConstOrValue::Value],
            &[0.25, 0.75],
            self.rng,
        ) {
            ConstOrValue::Const => {
                <BrilDist as Distribution<FuzzedConstInstr>>::sample(
                    &BrilDist, self.rng,
                )
                .0
            }
            ConstOrValue::Value => {
                <BrilDist as Distribution<FuzzedValueInstr>>::sample(
                    &BrilDist, self.rng,
                )
                .0
            }
        };
        if !matches!(instr, Instruction::Const(..)) {
            let mut operands = vec![];
            for _ in 0..instr.num_operands() {
                operands.push(self.sample_var_of_ty(instr.operands_ty())?);
            }
            instr.config_operands(operands);
        }
        let dest = self.alloc_next_var(instr.dest_ty());
        instr.config_dest(dest);
        Some(self.ast.insert(Ast::Instruction(instr)))
    }

    pub fn sample_block(&mut self) -> AstIdx {
        let block_size =
            (self.config.block_size_distr.sample(self.rng) as isize).abs();
        let mut block = vec![];
        // block_size not actually precise, we may end up insert a few more
        // const instrs
        for _ in 0..block_size {
            let next = loop {
                if let Some(instr) = self.sample_instr() {
                    break instr;
                }
                // backoff, sample const instr
                let mut new_const =
                    <BrilDist as Distribution<FuzzedConstInstr>>::sample(
                        &BrilDist, self.rng,
                    )
                    .0;
                let dest = self.alloc_next_var(new_const.dest_ty());
                new_const.config_dest(dest);
                block.push(self.ast.insert(Ast::Instruction(new_const)));
            };
            block.push(next);
        }
        self.ast.insert(Ast::Seq(block))
    }

    pub fn sample_loop(&mut self, layout: &AstLayout) -> Option<AstIdx> {
        if layout.num_blocks < 1 {
            return None;
        }
        let condition = self.sample_var_of_ty(Type::Bool)?;
        let body = self.sample_ast(layout);
        Some(self.ast.insert(Ast::Loop(condition, body)))
    }

    pub fn sample_if_else(&mut self, layout: &AstLayout) -> Option<AstIdx> {
        if layout.num_blocks < 2 {
            return None;
        }
        let condition = self.sample_var_of_ty(Type::Bool)?;
        let pre_branch_snapshot = self.snapshot();

        // don't allow empty block
        let if_num_block = (1..layout.num_blocks).choose(self.rng).unwrap();
        let if_ast = self.sample_ast(&AstLayout {
            max_block_depth: layout.max_block_depth,
            num_blocks: if_num_block,
        });
        let mut if_exit_snapshot = self.restore_snapshot(pre_branch_snapshot);

        let else_ast = self.sample_ast(&AstLayout {
            max_block_depth: layout.max_block_depth,
            num_blocks: layout.num_blocks - if_num_block,
        });
        let else_exit_snapshot = self.snapshot();
        if_exit_snapshot.merge(else_exit_snapshot);
        self.restore_snapshot(if_exit_snapshot);
        Some(self.ast.insert(Ast::If(condition, if_ast, else_ast)))
    }

    pub fn sample_ast(&mut self, layout: &AstLayout) -> AstIdx {
        enum AstNode {
            LeafBlock,
            IfElse,
            Loop,
        }

        let mut seq = vec![];
        let mut budget = layout.num_blocks;
        while budget > 0 {
            match sample_one_by_weights(
                &[AstNode::LeafBlock, AstNode::IfElse, AstNode::Loop],
                &[0.6, 0.2, 0.2],
                self.rng,
            ) {
                AstNode::LeafBlock => {
                    seq.push(self.sample_block());
                    budget -= 1;
                }
                AstNode::IfElse => {
                    if layout.max_block_depth == 0 || budget < 2 {
                        continue;
                    }
                    let subtree_layout = AstLayout {
                        max_block_depth: (0..layout.max_block_depth)
                            .choose(self.rng)
                            .unwrap(),
                        num_blocks: (2..=budget).choose(self.rng).unwrap(),
                    };
                    if let Some(if_else_ast) =
                        self.sample_if_else(&subtree_layout)
                    {
                        seq.push(if_else_ast);
                        budget -= subtree_layout.num_blocks;
                    }
                }
                AstNode::Loop => {
                    if layout.max_block_depth == 0 {
                        continue;
                    }
                    let subtree_layout = AstLayout {
                        max_block_depth: (0..layout.max_block_depth)
                            .choose(self.rng)
                            .unwrap(),
                        num_blocks: (1..=budget).choose(self.rng).unwrap(),
                    };
                    if let Some(loop_ast) = self.sample_loop(&subtree_layout) {
                        seq.push(loop_ast);
                        budget -= subtree_layout.num_blocks;
                    }
                }
            }
        }
        self.ast.insert(Ast::Seq(seq))
    }

    fn alloc_next_var(&mut self, ty: Type) -> Variable {
        let next = Variable(self.next_var as u32, ty);
        self.next_var += 1;
        self.live_vars.entry(ty).or_default().push(next);
        next
    }

    fn sample_var_of_ty(&mut self, ty: Type) -> Option<Variable> {
        self.live_vars
            .get(&ty)
            .and_then(|candidates| candidates.choose(self.rng).cloned())
    }

    fn snapshot(&self) -> SnapShot {
        SnapShot {
            next_var: self.next_var,
            live_vars: self.live_vars.clone(),
        }
    }

    fn restore_snapshot(&mut self, snapshot: SnapShot) -> SnapShot {
        let cur_snapshot = self.snapshot();
        self.next_var = snapshot.next_var;
        self.live_vars = snapshot.live_vars;
        cur_snapshot
    }
}
