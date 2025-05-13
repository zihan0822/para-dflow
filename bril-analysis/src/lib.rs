// Copyright (C) 2025 Zihan Li and Ethan Uppal.
pub mod analysis;
pub mod scc;


use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::SecondaryMap;

pub mod parallel;
pub mod sequential;

pub enum Direction {
    Forward,
    Backward,
}

fn construct_postorder(cfg: &Cfg) -> Vec<BasicBlockIdx> {
    fn helper(
        cfg: &Cfg,
        current: BasicBlockIdx,
        visited: &mut SecondaryMap<BasicBlockIdx, bool>,
        traversal: &mut Vec<BasicBlockIdx>,
    ) {
        visited.insert(current, true);
        for successor in cfg.successors(current) {
            if !visited.contains_key(successor) {
                helper(cfg, successor, visited, traversal);
            }
        }
        traversal.push(current);
    }

    let mut traversal = vec![];
    let mut visited = SecondaryMap::with_capacity(cfg.vertices.capacity());
    helper(cfg, cfg.entry, &mut visited, &mut traversal);
    traversal
}
