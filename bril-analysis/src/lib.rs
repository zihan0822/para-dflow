// Copyright (C) 2025 Zihan Li and Ethan Uppal.
pub mod analysis;
pub mod scc;

use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use scc::{Component, CondensedCfg};
use slotmap::SecondaryMap;

pub mod parallel;
pub mod sequential;

pub enum Direction {
    Forward,
    Backward,
}

trait TraverseCfgLike<'a> {
    type Context;

    fn entry(&self) -> BasicBlockIdx;

    fn vertices_capacity(&self) -> usize;

    fn successors(
        &self,
        context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx>;

    fn predecessors(
        &self,
        context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx>;
}

impl<'program> TraverseCfgLike<'_> for Cfg<'program> {
    type Context = ();

    fn entry(&self) -> BasicBlockIdx {
        self.entry
    }
    fn vertices_capacity(&self) -> usize {
        self.vertices.capacity()
    }

    fn successors(
        &self,
        _context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        Cfg::successors(self, current)
    }

    fn predecessors(
        &self,
        _context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        Cfg::predecessors(self, current)
    }
}

impl<'a> TraverseCfgLike<'a> for Component {
    type Context = CondensedCfg<'a, 'a>;

    fn entry(&self) -> BasicBlockIdx {
        self.entry
    }

    fn vertices_capacity(&self) -> usize {
        self.vertices.capacity()
    }

    fn successors(
        &self,
        context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        context.cfg.successors(current)
    }

    fn predecessors(
        &self,
        context: &Self::Context,
        current: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        context.cfg.predecessors(current)
    }
}

fn construct_postorder<'a, C: TraverseCfgLike<'a>>(
    cfg_like: &C,
    context: &C::Context,
) -> Vec<BasicBlockIdx> {
    fn helper<'a, C: TraverseCfgLike<'a>>(
        cfg_like: &C,
        context: &C::Context,
        current: BasicBlockIdx,
        visited: &mut SecondaryMap<BasicBlockIdx, bool>,
        traversal: &mut Vec<BasicBlockIdx>,
    ) {
        visited.insert(current, true);
        for successor in cfg_like.successors(context, current) {
            if !visited.contains_key(successor) {
                helper(cfg_like, context, successor, visited, traversal);
            }
        }
        traversal.push(current);
    }

    let mut traversal = vec![];
    let mut visited = SecondaryMap::with_capacity(cfg_like.vertices_capacity());
    helper(
        cfg_like,
        context,
        cfg_like.entry(),
        &mut visited,
        &mut traversal,
    );
    traversal
}
