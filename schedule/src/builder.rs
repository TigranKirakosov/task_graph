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

    /// Adds a dependency `a -> b` and increments `b`'s dependants
    pub(crate) fn add_dep(&mut self, lhs: InternId, rhs: InternId) {
        if self.adj[lhs].contains(&rhs) {
            return;
        }

        self.adj[lhs].push(rhs);
        self.in_deg[rhs] += 1;
    }

    /// Merges sub [Schedule] into this [Schedule] returning prior's shifted roots and leaves
    pub(crate) fn merge(&mut self, sub: Schedule<I, Building>) -> (Vec<InternId>, Vec<InternId>) {
        let offset = self.node_meta.len();

        let sub_roots = sub.roots().map(|id| id + offset).collect();
        let sub_leaves = sub.leaves().map(|id| id + offset).collect();

        self.node_meta.extend(sub.node_meta);
        self.in_deg.extend(sub.in_deg);

        for mut downstream in sub.adj {
            for node in &mut downstream {
                *node += offset;
            }

            self.adj.push(downstream);
        }

        (sub_roots, sub_leaves)
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
