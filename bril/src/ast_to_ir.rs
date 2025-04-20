use slotmap::SlotMap;

use crate::{
    ast::{Ast, AstIdx},
    builder::{BasicBlockIdx, FunctionBuilder},
    ir::Instruction,
};

pub fn ast_to_ir(
    ast: &SlotMap<AstIdx, Ast>,
    ast_root: AstIdx,
    builder: &mut FunctionBuilder,
    current_idx: BasicBlockIdx,
) {
    match &ast[ast_root] {
        Ast::Instruction(instruction) => {
            builder.block_mut(current_idx).push(instruction.clone())
        }
        Ast::If(condition, if_true, if_false) => {
            let if_true_block_idx = builder.new_block();
            ast_to_ir(&ast, *if_true, builder, if_true_block_idx);

            let if_false_block_idx = builder.new_block();
            ast_to_ir(&ast, *if_false, builder, if_false_block_idx);

            let branch = Instruction::Br(
                *condition,
                builder.block_label(if_true_block_idx),
                builder.block_label(if_false_block_idx),
            );
            builder.block_mut(current_idx).push(branch);
        }
        Ast::Seq(children) => {
            for child in children {
                ast_to_ir(&ast, *child, builder, current_idx);
            }
        }
    }
}
