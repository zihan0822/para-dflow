// Copyright (C) 2025 Zihan Li and Ethan Uppal.
use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::collections::HashSet;

new_key_type! { pub struct ComponentIdx; }

pub struct Component {
    pub vertices: Vec<BasicBlockIdx>,
    pub entry: BasicBlockIdx,
    /// count of back edges within the component
    pub num_back_edges: usize,
}

impl Component {
    pub fn contains(&self, query: BasicBlockIdx) -> bool {
        self.vertices.iter().any(|idx| query.eq(idx))
    }
}

pub struct CondensedCfg<'cfg, 'program> {
    pub cfg: &'cfg Cfg<'program>,
    pub entry: ComponentIdx,
    pub components: SlotMap<ComponentIdx, Component>,
    pub edges: SecondaryMap<ComponentIdx, Vec<ComponentIdx>>,
    pub rev_edges: SecondaryMap<ComponentIdx, Vec<ComponentIdx>>,
}

impl<'cfg, 'program> CondensedCfg<'cfg, 'program> {
    pub fn intra_comp_edges(
        &self,
        comp_idx: ComponentIdx,
        block_idx: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        if !self.components[comp_idx].contains(block_idx) {
            return vec![];
        }
        self.cfg
            .successors(block_idx)
            .into_iter()
            .filter(|successor| self.components[comp_idx].contains(*successor))
            .collect()
    }

    pub fn inter_comp_edges(
        &self,
        comp_idx: ComponentIdx,
        block_idx: BasicBlockIdx,
    ) -> Vec<BasicBlockIdx> {
        self.cfg
            .successors(block_idx)
            .into_iter()
            .filter(|successor| !self.components[comp_idx].contains(*successor))
            .collect()
    }

    /// tarjan algorithm for constructing strongly connected components
    pub fn from_cfg(cfg: &'cfg Cfg<'program>) -> CondensedCfg<'cfg, 'program> {
        struct Visitor<'a, 'program> {
            cfg: &'a Cfg<'program>,
            val: usize,
            lowest: SecondaryMap<BasicBlockIdx, usize>,
            preorder: SecondaryMap<BasicBlockIdx, usize>,
            back_edges_cnt: SecondaryMap<BasicBlockIdx, usize>,
            stack: Vec<BasicBlockIdx>,
            in_stack: HashSet<BasicBlockIdx>,
            components: SlotMap<ComponentIdx, Component>,
            block2comp: SecondaryMap<BasicBlockIdx, ComponentIdx>,
        }

        let mut visitor = Visitor {
            cfg,
            val: 0,
            lowest: SecondaryMap::with_capacity(cfg.vertices.capacity()),
            preorder: SecondaryMap::with_capacity(cfg.vertices.capacity()),
            back_edges_cnt: SecondaryMap::with_capacity(
                cfg.vertices.capacity(),
            ),
            stack: vec![],
            in_stack: HashSet::new(),
            components: SlotMap::with_key(),
            block2comp: SecondaryMap::with_capacity(cfg.vertices.capacity()),
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
                        *self
                            .back_edges_cnt
                            .entry(successor)
                            .unwrap()
                            .or_default() += 1;
                        lowest = lowest.min(self.preorder[successor]);
                    } else if !self.preorder.contains_key(successor) {
                        self.tarjan(successor);
                        lowest = lowest.min(self.lowest[successor]);
                    }
                }
                self.lowest.insert(current, lowest);
                if lowest == self.preorder[current] {
                    let mut vertices = vec![];
                    let mut num_back_edges = 0;
                    while let Some(v) = self.stack.pop() {
                        vertices.push(v);
                        if let Some(cnt) = self.back_edges_cnt.get(v) {
                            num_back_edges += cnt;
                        }
                        self.in_stack.remove(&v);
                        if v == current {
                            break;
                        }
                    }
                    let comp_idx = self.components.insert(Component {
                        entry: current,
                        vertices: vertices.clone(),
                        num_back_edges,
                    });
                    for block_idx in vertices {
                        self.block2comp.insert(block_idx, comp_idx);
                    }
                }
            }
        }
        visitor.tarjan(cfg.entry);
        let components = visitor.components;

        // build edges between components
        let mut edges = SecondaryMap::with_capacity(components.len());
        for (comp_idx, comp) in &components {
            let comp_successors: HashSet<_> = comp
                .vertices
                .iter()
                .flat_map(|block_idx| {
                    cfg.successors(*block_idx).into_iter().filter_map(
                        |successor| {
                            if !comp.contains(successor) {
                                Some(visitor.block2comp[successor])
                            } else {
                                None
                            }
                        },
                    )
                })
                .collect();
            edges.insert(comp_idx, Vec::from_iter(comp_successors));
        }
        let mut rev_edges =
            SecondaryMap::<ComponentIdx, Vec<ComponentIdx>>::with_capacity(
                components.len(),
            );
        for (source, out_edges) in &edges {
            for destination in out_edges {
                rev_edges
                    .entry(*destination)
                    .unwrap()
                    .or_default()
                    .push(source);
            }
        }
        let entry = visitor.block2comp[cfg.entry];
        Self {
            cfg,
            entry,
            components,
            edges,
            rev_edges,
        }
    }
}
