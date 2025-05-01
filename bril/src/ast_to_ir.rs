use slotmap::SlotMap;

use crate::{
    ast::{Ast, AstIdx},
    builder::{BasicBlockBuilder, FunctionBuilder, ProgramBuilder},
    ir::{Instruction, LabelIdx, Program},
};

pub fn ast_to_ir(ast: &SlotMap<AstIdx, Ast>, ast_root: AstIdx) -> Program {
    let mut program_builder = ProgramBuilder::new();
    let mut fn_builder = program_builder.new_function(String::from("main"));
    let last_block_builder = recursed_ast_to_ir(
        ast,
        ast_root,
        &mut fn_builder,
        &mut 0,
        BasicBlockBuilder::new(),
    );
    if !last_block_builder.is_empty() {
        fn_builder.seal_block(last_block_builder);
    }
    fn_builder.finish();
    program_builder.finish()
}

fn recursed_ast_to_ir(
    ast: &SlotMap<AstIdx, Ast>,
    ast_root: AstIdx,
    fn_builder: &mut FunctionBuilder,
    cur_if_else_idx: &mut usize,
    mut block_builder: BasicBlockBuilder,
) -> BasicBlockBuilder {
    match &ast[ast_root] {
        Ast::Instruction(instr) => {
            block_builder.add_instr(instr.clone());
            block_builder
        }
        Ast::If(condition, if_true, if_false) => {
            let true_block_label = format!("if.else.{}.true", *cur_if_else_idx);
            let false_block_label =
                format!("if.else.{}.false", *cur_if_else_idx);
            let exit_label = format!("if.else.{}.exit", *cur_if_else_idx);

            *cur_if_else_idx += 1;
            block_builder.add_patched_instr(
                Instruction::Br(*condition, LabelIdx(0), LabelIdx(0)),
                vec![true_block_label.clone(), false_block_label.clone()],
            );
            fn_builder.seal_block(block_builder);

            let mut true_block_builder = recursed_ast_to_ir(
                ast,
                *if_true,
                fn_builder,
                cur_if_else_idx,
                BasicBlockBuilder::with_label(true_block_label),
            );
            true_block_builder.add_patched_instr(
                Instruction::Jmp(LabelIdx(0)),
                vec![exit_label.clone()],
            );
            fn_builder.seal_block(true_block_builder);

            let mut false_block_builder = recursed_ast_to_ir(
                ast,
                *if_false,
                fn_builder,
                cur_if_else_idx,
                BasicBlockBuilder::with_label(false_block_label),
            );
            false_block_builder.add_patched_instr(
                Instruction::Jmp(LabelIdx(0)),
                vec![exit_label.clone()],
            );
            fn_builder.seal_block(false_block_builder);
            BasicBlockBuilder::with_label(exit_label)
        }
        Ast::Seq(children) => {
            for child in children {
                block_builder = recursed_ast_to_ir(
                    ast,
                    *child,
                    fn_builder,
                    cur_if_else_idx,
                    block_builder,
                );
            }
            block_builder
        }
    }
}
