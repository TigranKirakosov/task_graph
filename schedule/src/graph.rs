use std::collections::VecDeque;

use crate::schedule::{ExternId, GraphError, InternId, Schedule};

impl<I, S> Schedule<I, S>
where
    I: ExternId,
    S: 'static,
{
    /// Kahn's topological sort
    pub(crate) fn sort_ordered(&self) -> Result<Vec<InternId>, GraphError> {
        let mut order = Vec::new();
        let mut in_deg = self.in_deg.clone();
        let mut q = VecDeque::<InternId>::from(self.roots().collect::<Vec<_>>());

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

    pub(crate) fn roots(&self) -> impl Iterator<Item = InternId> {
        let len = self.node_meta.len();
        (0..len).filter(|&id| self.in_deg[id] == 0)
    }

    pub(crate) fn leaves(&self) -> impl Iterator<Item = InternId> {
        let len = self.node_meta.len();
        (0..len).filter(|&id| self.adj[id].is_empty())
    }
}
