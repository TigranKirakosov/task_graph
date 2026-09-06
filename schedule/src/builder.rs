use std::collections::{HashMap, HashSet};

use crate::lifecycle::Running;
use crate::meta::Meta;
use crate::schedule::{ExternId, InternId, Schedule, TaskMarker};

pub(crate) struct Building;

impl<I> Schedule<I, Building>
where
    I: ExternId,
{
    pub(crate) fn new() -> Self {
        Self {
            listeners: std::collections::HashMap::new(),
            extern_to_local: std::collections::HashMap::new(),
            local_to_extern: Vec::new(),
            in_deg: Vec::new(),
            adj: Vec::new(),
            node_meta: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn add_task<T: TaskMarker>(&mut self) -> InternId {
        self.node_meta.push(Meta::new::<T>());
        self.adj.push(vec![]);
        self.in_deg.push(0);

        self.node_meta.len() - 1
    }

    /// Adds a dependency `a -> b` and increments `b`'s dependants count
    pub(crate) fn add_dep(&mut self, lhs: InternId, rhs: InternId) {
        if self.adj[lhs].contains(&rhs) {
            return;
        }

        self.adj[lhs].push(rhs);
        self.in_deg[rhs] += 1;
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
    pub(crate) fn merge(&mut self, sub: Schedule<I, Building>, at: Vec<InternId>) -> Vec<InternId> {
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
            self.in_deg[nbr] = self.in_deg[nbr].saturating_sub(count);
        }

        // Offset root and leaf indices of sub
        // so they stand right after last node of this graph
        let offset = self.node_meta.len();
        let sub_roots: Vec<InternId> = sub.roots().map(|id| id + offset).collect();
        let sub_leaves: Vec<InternId> = sub.leaves().map(|id| id + offset).collect();

        // Extend with sub vectors
        self.node_meta.extend(sub.node_meta);
        self.in_deg.extend(sub.in_deg);

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
                self.in_deg[sub_root] += 1;
            }
        }

        // Stitch sub's leaf nodes with downstream neighbors of at nodes
        // while restoring their in-degrees
        for &sub_leaf in &sub_leaves {
            for &nbr in at_downstream.keys() {
                self.adj[sub_leaf].push(nbr);
                self.in_deg[nbr] += 1;
            }
        }

        sub_leaves
    }

    pub(crate) fn build(mut self, provider: fn(&Meta) -> I) -> Schedule<I, Running> {
        for (node_id, meta) in self.node_meta.iter().enumerate() {
            let extern_id = provider(meta);
            self.local_to_extern.push(extern_id.clone());
            self.extern_to_local.insert(extern_id, node_id);
        }

        Schedule {
            listeners: self.listeners,
            extern_to_local: self.extern_to_local,
            local_to_extern: self.local_to_extern,
            adj: self.adj,
            in_deg: self.in_deg,
            node_meta: self.node_meta,
            _marker: std::marker::PhantomData,
        }
    }
}
