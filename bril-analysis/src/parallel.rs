// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use dashmap::DashMap;
use fixedbitset::FixedBitSet;
use rayon::Scope;
use std::{collections::VecDeque, sync::Arc};

use bril::builder::BasicBlockIdx;
use bril_cfg::Cfg;
use slotmap::SecondaryMap;

use crate::{
    Direction, construct_postorder,
    scc::{ComponentIdx, CondensedCfg},
};

pub fn solve_dataflow(
    cfg: &Cfg,
    direction: Direction,
    entry_inputs: FixedBitSet,
    merge: impl Fn(FixedBitSet, &FixedBitSet) -> FixedBitSet,
    transfer: impl Fn(BasicBlockIdx, FixedBitSet) -> FixedBitSet,
    threads: usize,
) -> SecondaryMap<BasicBlockIdx, FixedBitSet> {
    // this will be on a per component basis:
    // let postorder_traversal = construct_postorder(cfg);
    // let blocks = match direction {
    //     Direction::Forward => {
    //         VecDeque::from_iter(postorder_traversal.into_iter().rev())
    //     }
    //     Direction::Backward => VecDeque::from_iter(postorder_traversal),
    // };

    let condensed_cfg = CondensedCfg::from_cfg(cfg);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    fn dependencies(
        condensed_cfg: &CondensedCfg,
        direction: Direction,
        current: ComponentIdx,
    ) -> Vec<ComponentIdx> {
        match direction {
            Direction::Forward => condensed_cfg
                .rev_edges
                .get(current)
                .cloned()
                .unwrap_or_default(),
            Direction::Backward => condensed_cfg
                .edges
                .get(current)
                .cloned()
                .unwrap_or_default(),
        }
    }

    let mut dependencies_left = DashMap::new();
    for component_idx in condensed_cfg.components.keys() {
        dependencies_left.insert(
            component_idx,
            dependencies(&condensed_cfg, direction, component_idx).len(),
        );
    }
    let mut dependencies_left = Arc::new(dependencies_left);

    let mut starting_set = vec![];
    match direction {
        Direction::Forward => {
            starting_set.push(condensed_cfg.entry);
        }
        Direction::Backward => {
            let mut bfs = VecDeque::from_iter([condensed_cfg.entry]);
            let mut frontier = vec![];
            while let Some(next) = bfs.pop_front() {
                let neighbors =
                    condensed_cfg.edges.get(next).cloned().unwrap_or_default();
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

    // TODO: store results of dataflow per component in some DashMap

    fn worker<'scope>(
        scope: &Scope<'scope>,
        starting_component: ComponentIdx,
        condensed_cfg_edges: &'scope SecondaryMap<
            ComponentIdx,
            Vec<ComponentIdx>,
        >,
        dependencies_left: Arc<DashMap<ComponentIdx, usize>>,
    ) {
        // sequential dataflow

        for &dependent in condensed_cfg_edges
            .get(starting_component)
            .unwrap_or(&vec![])
        {
            let mut remaining = dependencies_left.entry(dependent).or_default();
            if *remaining > 0 {
                *remaining -= 1;
                if *remaining == 0 {
                    let dependencies_left = dependencies_left.clone();
                    scope.spawn(move |scope| {
                        worker(
                            scope,
                            dependent,
                            condensed_cfg_edges,
                            dependencies_left,
                        );
                    });
                }
            }
        }
    }

    let condensed_cfg_edges = &condensed_cfg.edges;
    pool.scope(move |scope| {
        for starting_component in starting_set {
            let mut dependencies_left = dependencies_left.clone();
            worker(
                scope,
                starting_component,
                condensed_cfg_edges,
                dependencies_left,
            );
        }
    });

    todo!()
}
