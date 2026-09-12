use crate::{NodeStatus, Resolution};
use action_orc_core::*;

#[derive(Default)]
pub(crate) struct Schedule {
    pub(crate) in_degree: Vec<usize>,
}

impl Schedule {
    pub(crate) fn from(graph: &Graph) -> Self {
        Self {
            in_degree: graph.in_degree().to_vec(),
        }
    }

    pub(crate) fn start(&self, graph: &Graph) -> Vec<(NodeId, NodeStatus)> {
        graph
            .sources()
            .map(|source| (source, NodeStatus::Started))
            .collect()
    }

    pub(crate) fn advance(
        &mut self,
        graph: &Graph,
        target: NodeId,
        resolution: Resolution,
    ) -> Vec<(NodeId, NodeStatus)> {
        let mut queue = vec![(target, NodeStatus::Resolved(resolution))];

        // Resolve downstream nodes
        for &ds in &graph.adj()[target] {
            self.in_degree[ds] = self.in_degree[ds].saturating_sub(1);
            if self.in_degree[ds] == 0 {
                queue.push((ds, NodeStatus::Started));
            }
        }

        queue
    }
}
