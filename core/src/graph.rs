use std::collections::{HashMap, VecDeque};

use crate::meta::{Marker, Meta};

pub type NodeId = usize;

#[derive(Debug, PartialEq)]
pub enum GraphError {
    CycleDetected,
}

#[derive(Default)]
pub struct Graph {
    pub(crate) in_degree: Vec<usize>,
    pub(crate) adj: Vec<Vec<NodeId>>,
    pub(crate) meta: Vec<Meta>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node<T: Marker>(&mut self) -> NodeId {
        self.meta.push(Meta::new::<T>());
        self.adj.push(vec![]);
        self.in_degree.push(0);

        self.adj.len() - 1
    }

    /// Adds a dependency `a -> b` and increments `b`'s dependants count
    pub fn add_edge(&mut self, lhs: NodeId, rhs: NodeId) {
        if self.adj[lhs].contains(&rhs) {
            return;
        }

        self.adj[lhs].push(rhs);
        self.in_degree[rhs] += 1;
    }

    /// Merges sub-[Schedule] into this [Schedule] returning shifted leaves of sub
    ///
    /// ### Single entry
    /// Merging sub-graph **H** (`x -> y`) into graph **G** (`a -> b`) at **G**(`a`):
    /// ```text
    /// [a] ──> [x] ──> [y] ──> [b]
    /// ```
    ///
    /// ### Multiple entries
    /// Merging sub-graph **H** (`x`) into graph **G** (`a | b -> c`) at **G**(`a | b`):
    /// ```text
    /// [a] ──┐
    ///       ├──> [x] ──> [c]
    /// [b] ──┘
    /// ```
    pub fn merge(&mut self, sub: Graph, at: Vec<NodeId>) -> Vec<NodeId> {
        // Collect unique downstream neighbours of each node of `at` list
        // while counting broken edges
        let mut at_downstream = HashMap::new();
        for &node in &at {
            let neighbours = std::mem::take(&mut self.adj[node]);
            for nbr in neighbours {
                *at_downstream.entry(nbr).or_insert(0) += 1;
            }
        }

        // Decrease in-degree for each collected neighbour by broken edges count
        for (&nbr, &count) in &at_downstream {
            self.in_degree[nbr] = self.in_degree[nbr].saturating_sub(count);
        }

        // Offset root and leaf indices of sub
        // so they stand right after last node of this graph
        let offset = self.adj.len();
        let sub_roots: Vec<NodeId> = sub.roots().map(|id| id + offset).collect();
        let sub_leaves: Vec<NodeId> = sub.leaves().map(|id| id + offset).collect();

        // Extend with sub vectors
        self.meta.extend(sub.meta);
        self.in_degree.extend(sub.in_degree);

        for mut downstream in sub.adj {
            // Offset every downstream node index aswell
            for node in &mut downstream {
                *node += offset;
            }

            self.adj.push(downstream);
        }

        // Stitch at nodes with sub's root nodes
        for &node in &at {
            for &sub_root in &sub_roots {
                self.adj[node].push(sub_root);
                self.in_degree[sub_root] += 1;
            }
        }

        // Stitch sub's leaf nodes with downstream neighbors of at nodes
        // while restoring their in-degrees
        for &sub_leaf in &sub_leaves {
            for &nbr in at_downstream.keys() {
                self.adj[sub_leaf].push(nbr);
                self.in_degree[nbr] += 1;
            }
        }

        sub_leaves
    }

    /// Kahn's topological sort
    pub fn sort_ordered(&self) -> Result<Vec<NodeId>, GraphError> {
        let mut order = Vec::new();
        let mut in_deg = self.in_degree.clone();
        let mut q = VecDeque::<NodeId>::from(self.roots().collect::<Vec<_>>());

        while let Some(id) = q.pop_front() {
            order.push(id);
            for &nbr in &self.adj[id] {
                in_deg[nbr] = in_deg[nbr].saturating_sub(1);
                if in_deg[nbr] == 0 {
                    q.push_back(nbr);
                }
            }
        }

        if order.len() != in_deg.len() {
            return Err(GraphError::CycleDetected);
        }

        Ok(order)
    }

    pub fn in_degree(&self) -> &[usize] {
        self.in_degree.as_slice()
    }

    pub fn adj(&self) -> &[Vec<NodeId>] {
        self.adj.as_slice()
    }

    pub fn meta(&self) -> &[Meta] {
        self.meta.as_slice()
    }

    pub fn roots(&self) -> impl Iterator<Item = NodeId> {
        let len = self.adj.len();
        (0..len).filter(|&id| self.in_degree[id] == 0)
    }

    pub fn leaves(&self) -> impl Iterator<Item = NodeId> {
        let len = self.adj.len();
        (0..len).filter(|&id| self.adj[id].is_empty())
    }
}
