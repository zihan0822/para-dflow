// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use dashmap::DashMap;
use fixedbitset::FixedBitSet;
use rayon::Scope;
use std::collections::VecDeque;

use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;

use crate::{
    Direction,
    scc::{ComponentIdx, CondensedCfg},
    sequential,
};

pub fn solve_dataflow(
    cfg: &Cfg,
    direction: Direction,
    entry_inputs: FixedBitSet,
    merge: impl Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet + Sync,
    transfer: impl Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet + Sync,
    threads: usize,
) -> DashMap<BasicBlockIdx, FixedBitSet> {
    let solver = ParallelSolver {
        condensed_cfg: CondensedCfg::from_cfg(cfg),
        pool: rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap(),
        entry_inputs,
        direction,
        merge,
        transfer,
        solution: DashMap::new(),
    };

    solver.solve();
    solver.solution
}

struct ParallelSolver<'cfg, M, T>
where
    M: Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet + Sync,
    T: Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet + Sync,
{
    condensed_cfg: CondensedCfg<'cfg, 'cfg>,
    pool: rayon::ThreadPool,
    entry_inputs: FixedBitSet,
    direction: Direction,
    merge: M,
    transfer: T,
    solution: DashMap<BasicBlockIdx, FixedBitSet>,
}

impl<M, T> ParallelSolver<'_, M, T>
where
    M: Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet + Sync,
    T: Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet + Sync,
{
    fn worker<'scope, 'a: 'scope>(
        &'a self,
        scope: &Scope<'scope>,
        current: ComponentIdx,
        dependencies_left: &'scope DashMap<ComponentIdx, usize>,
    ) {
        let component_entry_block =
            self.condensed_cfg.components[current].entry;
        let component = &self.condensed_cfg.components[current];
        let initial_in = {
            let predecessors: Vec<_> = match self.direction {
                Direction::Forward => self
                    .condensed_cfg
                    .cfg
                    .predecessors(component_entry_block)
                    .into_iter()
                    .filter(|&pred| !component.contains(pred))
                    .collect(),
                Direction::Backward => self
                    .condensed_cfg
                    .cfg
                    .successors(component_entry_block)
                    .into_iter()
                    .filter(|&succ| !component.contains(succ))
                    .collect(),
            };
            predecessors
                .iter()
                .map(|pred| self.solution.get(pred).unwrap().clone())
                .reduce(|in1, in2| (self.merge)(in1, &in2))
                .unwrap_or(self.entry_inputs.clone())
        };

        // sequential dataflow
        let partial_solution = sequential::solve_dataflow(
            &self.condensed_cfg.components[current],
            &self.condensed_cfg,
            self.direction,
            initial_in,
            &self.merge,
            &self.transfer,
        );

        for (block_idx, v) in partial_solution {
            self.solution.insert(block_idx, v);
        }

        for &dependent in
            self.condensed_cfg.edges.get(current).unwrap_or(&vec![])
        {
            let mut remaining = dependencies_left.entry(dependent).or_default();
            if *remaining > 0 {
                *remaining -= 1;
                if *remaining == 0 {
                    scope.spawn(move |scope| {
                        self.worker(scope, dependent, dependencies_left);
                    });
                }
            }
        }
    }

    fn dependencies(&self, current: ComponentIdx) -> Vec<ComponentIdx> {
        match self.direction {
            Direction::Forward => self
                .condensed_cfg
                .rev_edges
                .get(current)
                .cloned()
                .unwrap_or_default(),
            Direction::Backward => self
                .condensed_cfg
                .edges
                .get(current)
                .cloned()
                .unwrap_or_default(),
        }
    }

    fn solve(&self) {
        let dependencies_left = DashMap::new();
        for component_idx in self.condensed_cfg.components.keys() {
            dependencies_left
                .insert(component_idx, self.dependencies(component_idx).len());
        }

        let mut starting_set = vec![];
        match self.direction {
            Direction::Forward => {
                starting_set.push(self.condensed_cfg.entry);
            }
            Direction::Backward => {
                let mut bfs = VecDeque::from_iter([self.condensed_cfg.entry]);
                let mut frontier = vec![];
                while let Some(next) = bfs.pop_front() {
                    let neighbors = self
                        .condensed_cfg
                        .edges
                        .get(next)
                        .cloned()
                        .unwrap_or_default();
                    if neighbors.is_empty() {
                        frontier.push(next);
                    } else {
                        for neighbor in neighbors {
                            bfs.push_back(neighbor);
                        }
                    }
                }
                starting_set = frontier;
            }
        }

        let dependencies_left = &dependencies_left;

        self.pool.scope(move |scope| {
            for starting_component in starting_set {
                self.worker(scope, starting_component, dependencies_left);
            }
        });
    }
}
