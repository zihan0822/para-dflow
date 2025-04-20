use crate::dist::*;
use bril_rs::program::*;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::DfsPostOrder;
use rand::Rng;
use rand::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

thread_local! {
    static RNG: RefCell<rand::rngs::ThreadRng> = RefCell::new(rand::rng());
}
const MAX_FAN_OUT: usize = 2;

type TreeInner = DiGraph<usize, ()>;
pub struct Tree {
    pub root: NodeIndex,
    pub inner: TreeInner,
}

pub fn cfg_gen() -> DiGraph<usize, ()> {
    RNG.with_borrow_mut(generate_reducible_cfg)
}

pub fn generate_bril_program(num_fns: usize) -> Program {
    Program {
        functions: (0..num_fns)
            .map(|_| {
                let prototype = RNG.with_borrow_mut(|rng| rng.sample(BrilDist));
                generate_fn(8, prototype)
            })
            .collect(),
    }
}

fn generate_fn(num_instrs: usize, prototype: Prototype) -> Function {
    let instrs = generate_code_blk(num_instrs, Context::from_prototype(&prototype));
    Function {
        args: prototype.args,
        instrs,
        name: prototype.name,
        return_type: prototype.return_type,
    }
}

fn generate_code_blk(num_instrs: usize, mut ctx: Context) -> Vec<Code> {
    let mut instrs = vec![];
    #[derive(Sample)]
    enum BoolOrArith {
        #[w = 0.2]
        Bool(BoolInst),
        #[w = 0.8]
        Arith(ArithInst),
    }

    for _ in 0..num_instrs {
        let next = match RNG.with_borrow_mut(|rng| BoolOrArith::sample_with_ctx(&ctx, rng)) {
            BoolOrArith::Bool(bool_instr) => bool_instr.0,
            BoolOrArith::Arith(arith_instr) => arith_instr.0,
        };
        let (dest, op_type) = parse_dest_and_ty(&next);
        ctx.insert_new_local_var(dest, op_type);
        instrs.push(Code::Instruction(next));
    }
    instrs
}

fn parse_dest_and_ty(instr: &Instruction) -> (String, Type) {
    match instr {
        Instruction::Value { dest, op_type, .. } => (dest.clone(), op_type.clone()),
        Instruction::Constant {
            dest, const_type, ..
        } => (dest.clone(), const_type.clone()),
        _ => unreachable!(),
    }
}

fn generate_reducible_cfg<R: Rng + ?Sized>(rng: &mut R) -> DiGraph<usize, ()> {
    let tree = generate_random_tree(16, 3, rng);
    let mut cfg = tree.inner.clone();
    add_random_cross_and_forward_edges(&mut cfg, &tree, rng);
    add_random_back_edges(&mut cfg, tree.root, rng);
    cfg
}


fn add_random_cross_and_forward_edges<R: Rng + ?Sized>(
    cfg: &mut DiGraph<usize, ()>,
    tree: &Tree,
    rng: &mut R,
) {
    let desc = find_descendants(tree);

    for node in tree.inner.node_indices() {
        let num_added = (0..=2).choose(rng).unwrap();
        for to in siblings(&tree.inner, node)
            .into_iter()
            .chain(Some(node))
            .flat_map(|sib| desc.get(&sib).cloned().unwrap())
            .filter(|nx| tree.inner.find_edge(node, *nx).is_none())
            .chain(siblings(&tree.inner, node))
            .choose_multiple(rng, num_added)
        {
            cfg.add_edge(node, to, ());
        }
    }

    for node in cfg.node_indices() {
        let out_degree = cfg.neighbors(node).count();
        if out_degree > MAX_FAN_OUT {
            let to_remove = cfg
                .neighbors(node)
                .filter(|to| {
                    cfg.neighbors_directed(*to, petgraph::Direction::Incoming)
                        .count()
                        >= 2
                })
                .choose_multiple(rng, out_degree - MAX_FAN_OUT);
            for nx in to_remove {
                let edge_idx = cfg.find_edge(node, nx).unwrap();
                cfg.remove_edge(edge_idx);
            }
        }
    }
}

fn add_random_back_edges<R: Rng + ?Sized>(
    cfg: &mut DiGraph<usize, ()>,
    entry: NodeIndex,
    rng: &mut R,
) {
    let dominators = get_dominators(cfg, entry);
    for node in cfg.node_indices() {
        let out_degree = cfg.neighbors(node).count();
        // we don't add extra backedge to potential exit node
        if out_degree < MAX_FAN_OUT && out_degree > 0{
            let num_added = (0..=MAX_FAN_OUT - out_degree).choose(rng).unwrap();
            for nx in dominators
                .get(&node)
                .unwrap()
                .iter()
                .choose_multiple(rng, num_added)
            {
                cfg.add_edge(node, *nx, ());
            }
        }
    }
}

fn siblings(tree: &DiGraph<usize, ()>, node: NodeIndex) -> Vec<NodeIndex> {
    let mut parent = tree.neighbors_directed(node, petgraph::Direction::Incoming);
    assert!(parent.clone().count() <= 1);
    parent
        .next()
        .map(|parent| tree.neighbors(parent).filter(|nx| *nx != node).collect())
        .unwrap_or_default()
}

fn find_descendants(tree: &Tree) -> HashMap<NodeIndex, HashSet<NodeIndex>> {
    let mut desc: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();
    let mut visit = DfsPostOrder::new(&tree.inner, tree.root);
    while let Some(nx) = visit.next(&tree.inner) {
        let neighbors = tree.inner.neighbors(nx);
        let joined_from_child: HashSet<NodeIndex> = neighbors
            .clone()
            .flat_map(|neighbor| desc.get(&neighbor).cloned().unwrap())
            .chain(neighbors)
            .collect();
        assert!(desc.insert(nx, joined_from_child).is_none())
    }
    desc
}

pub fn generate_random_tree<R: Rng + ?Sized>(
    num_nodes: usize,
    max_fan_out: usize,
    rng: &mut R,
) -> Tree {
    let mut tree = DiGraph::<usize, ()>::new();
    struct RandomTreeGen<'a, R: Rng + ?Sized> {
        max_fan_out: usize,
        rng: &'a mut R,
        tree: &'a mut TreeInner,
    }
    impl<R: Rng + ?Sized> RandomTreeGen<'_, R> {
        fn recurse_on_subtree(
            &mut self,
            inorder_range: std::ops::Range<usize>,
        ) -> Option<NodeIndex> {
            if inorder_range.is_empty() {
                return None;
            }
            let (mut start, end) = (inorder_range.start, inorder_range.end);
            let num_nodes = end - start;
            let root_val = inorder_range.choose(self.rng).unwrap();
            let root = self.tree.add_node(root_val);
            let num_subtree = self.rng.random_range(1..=self.max_fan_out.min(num_nodes));

            let subtree_size_split: Vec<usize> = {
                let mut interval: Vec<usize> = std::iter::once(0)
                    .chain((1..=num_nodes - 1).choose_multiple(self.rng, num_subtree - 1))
                    .chain(std::iter::once(num_nodes))
                    .collect();
                interval.sort();
                interval.windows(2).map(|w| w[1] - w[0]).collect()
            };
            for subtree_size in subtree_size_split {
                if start <= root_val && root_val < start + subtree_size {
                    if let Some(child) = self.recurse_on_subtree(start..root_val) {
                        self.tree.add_edge(root, child, ());
                    }
                    if let Some(child) = self.recurse_on_subtree(root_val + 1..start + subtree_size)
                    {
                        self.tree.add_edge(root, child, ());
                    }
                } else {
                    let child = self
                        .recurse_on_subtree(start..start + subtree_size)
                        .unwrap();
                    self.tree.add_edge(root, child, ());
                }
                start += subtree_size;
            }
            Some(root)
        }
    }
    let mut generator = RandomTreeGen {
        max_fan_out,
        rng,
        tree: &mut tree,
    };
    let root = generator.recurse_on_subtree(0..num_nodes).unwrap();
    Tree { root, inner: tree }
}

fn get_dominators(
    graph: &DiGraph<usize, ()>,
    entry: NodeIndex,
) -> HashMap<NodeIndex, HashSet<NodeIndex>> {
    let mut ret: HashMap<_, _> = graph
        .node_indices()
        .map(|node| {
            if node == entry {
                (node, HashSet::from_iter([entry]))
            } else {
                (node, HashSet::from_iter(graph.node_indices()))
            }
        })
        .collect();
    let mut worklist = VecDeque::from_iter(graph.node_indices());
    while !worklist.is_empty() {
        let item = worklist.pop_front().unwrap();
        let Some(mut dominator) = graph
            .neighbors_directed(item, petgraph::Direction::Incoming)
            .map(|parent| ret.get(&parent).cloned().unwrap())
            .reduce(|dom1, dom2| dom1.intersection(&dom2).copied().collect())
        else {
            continue;
        };
        dominator.insert(item);
        if !dominator.eq(ret.get(&item).unwrap()) {
            worklist.extend(graph.neighbors(item));
            ret.insert(item, dominator);
        }
    }
    ret
}
