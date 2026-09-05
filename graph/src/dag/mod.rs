use std::{any::TypeId, collections::VecDeque};

#[cfg(test)]
mod tests;

type NodeId = usize;

pub(crate) trait ExternId: std::hash::Hash + Eq + Clone + Send + Sync + 'static {}
impl<T: std::hash::Hash + Eq + Clone + Send + Sync + 'static> ExternId for T {}

pub(crate) struct Meta<I>
where
    I: ExternId,
{
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) extern_id: Option<I>,
}

impl<I: ExternId> Meta<I> {
    pub(crate) fn new<M: 'static>() -> Self {
        let full_name = std::any::type_name::<M>();
        let type_name = full_name
            .rsplit_once("::")
            .map(|(_, name)| name)
            .unwrap_or(full_name);

        Self {
            type_id: TypeId::of::<M>(),
            type_name,
            extern_id: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct DAG<I>
where
    I: ExternId,
{
    in_deg: Vec<usize>,
    adj: Vec<Vec<NodeId>>,
    node_meta: Vec<Meta<I>>,
}

#[derive(Debug, PartialEq)]
enum SortingError {
    CycleDetected,
}

impl<I> DAG<I>
where
    I: ExternId,
{
    pub(crate) fn add_node<M: 'static>(&mut self) -> NodeId {
        self.node_meta.push(Meta::new::<M>());
        self.adj.push(vec![]);
        self.in_deg.push(0);

        self.node_meta.len() - 1
    }

    /// Adds an edge `a -> b` and increments `b`'s in_degree
    pub(crate) fn add_edge(&mut self, lhs: NodeId, rhs: NodeId) {
        if self.adj[lhs].contains(&rhs) {
            return;
        }

        self.adj[lhs].push(rhs);
        self.in_deg[rhs] += 1;
    }

    /// Merges sub [DAG] into this [DAG] returning prior's shifted roots and leaves
    pub(crate) fn merge(&mut self, sub: DAG<I>) -> (Vec<NodeId>, Vec<NodeId>) {
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

    pub(crate) fn topological_sort(&self) -> Result<Vec<NodeId>, SortingError> {
        let mut order = Vec::new();
        let mut in_deg = self.in_deg.clone();
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
            return Err(SortingError::CycleDetected);
        }

        Ok(order)
    }

    pub(crate) fn roots(&self) -> impl Iterator<Item = NodeId> {
        let len = self.node_meta.len();
        (0..len).filter(|&id| self.in_deg[id] == 0)
    }

    pub(crate) fn leaves(&self) -> impl Iterator<Item = NodeId> {
        let len = self.node_meta.len();
        (0..len).filter(|&id| self.adj[id].is_empty())
    }
}
