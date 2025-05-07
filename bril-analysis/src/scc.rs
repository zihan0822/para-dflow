// Copyright (C) 2025 Zihan Li and Ethan Uppal.
use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::collections::HashSet;

new_key_type! {pub struct ComponentIdx; }

pub struct Component {
    pub vertices: Vec<BasicBlockIdx>,
    pub entry: BasicBlockIdx,
}

pub struct CondensedCfg<'program> {
    pub cfg: Cfg<'program>,
    pub components: SlotMap<ComponentIdx, Component>,
}

impl<'program> CondensedCfg<'program> {
    /// tarjan algorithm for constructing strongly connected components
    pub fn from_cfg(cfg: Cfg<'program>) -> CondensedCfg<'program> {
        struct Visitor<'a, 'program> {
            cfg: &'a Cfg<'program>,
            val: usize,
            lowest: SecondaryMap<BasicBlockIdx, usize>,
            preorder: SecondaryMap<BasicBlockIdx, usize>,
            stack: Vec<BasicBlockIdx>,
            in_stack: HashSet<BasicBlockIdx>,
            components: SlotMap<ComponentIdx, Component>,
        }

        let mut visitor = Visitor {
            cfg: &cfg,
            val: 0,
            lowest: SecondaryMap::with_capacity(cfg.vertices.capacity()),
            preorder: SecondaryMap::with_capacity(cfg.vertices.capacity()),
            stack: vec![],
            in_stack: HashSet::new(),
            components: SlotMap::with_key(),
        };

        impl Visitor<'_, '_> {
            fn tarjan(&mut self, current: BasicBlockIdx) {
                if self.preorder.contains_key(current) {
                    return;
                }

                self.preorder.insert(current, self.val);
                let mut lowest = self.val;
                self.val += 1;
                self.stack.push(current);
                self.in_stack.insert(current);

                for successor in self.cfg.successors(current) {
                    if self.in_stack.contains(&successor) {
                        lowest = lowest.min(self.preorder[successor]);
                    } else if !self.preorder.contains_key(successor) {
                        self.tarjan(successor);
                        lowest = lowest.min(self.lowest[successor]);
                    }
                }
                self.lowest.insert(current, lowest);
                if lowest == self.preorder[current] {
                    let mut vertices = vec![];
                    while let Some(v) = self.stack.pop() {
                        vertices.push(v);
                        self.in_stack.remove(&v);
                        if v == current {
                            break;
                        }
                    }
                    self.components.insert(Component {
                        entry: current,
                        vertices,
                    });
                }
            }
        }
        visitor.tarjan(cfg.entry);
        let components = visitor.components;
        Self { cfg, components }
    }
}
