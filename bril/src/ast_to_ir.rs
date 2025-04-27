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
    mut block_builder: BasicBlockBuilder,
) -> BasicBlockBuilder {
    match &ast[ast_root] {
        Ast::Instruction(instr) => {
            block_builder.add_instr(instr.clone());
            block_builder
        }
        Ast::If(condition, if_true, if_false) => {
            block_builder.add_instr(Instruction::Br(
                *condition,
                LabelIdx(0),
                LabelIdx(0),
            ));
            let block_idx = fn_builder.seal_block(block_builder);

            let true_block_builder = recursed_ast_to_ir(
                ast,
                *if_true,
                fn_builder,
                BasicBlockBuilder::new(),
            );
            let true_block_idx = fn_builder.seal_block(true_block_builder);
            let true_label_idx = fn_builder.block_label(true_block_idx);

            let false_block_builder = recursed_ast_to_ir(
                ast,
                *if_false,
                fn_builder,
                BasicBlockBuilder::new(),
            );
            let false_block_idx = fn_builder.seal_block(false_block_builder);
            let false_label_idx = fn_builder.block_label(false_block_idx);

            if let Instruction::Br(_, if_true, if_false) =
                fn_builder.block_tail_mut(block_idx)
            {
                *if_true = true_label_idx;
                *if_false = false_label_idx;
            }
            BasicBlockBuilder::new()
        }
        Ast::Seq(children) => {
            for child in children {
                block_builder =
                    recursed_ast_to_ir(ast, *child, fn_builder, block_builder);
            }
            block_builder
        }
    }
}
