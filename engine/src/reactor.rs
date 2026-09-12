use action_orc_core::*;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

use crate::{NodeStatus, Resolution, schedule::Schedule};

#[derive(Debug)]
pub enum ReactorError {
    MissingListener,
    UnknownTypeId,
}

pub struct Reactor {
    pub(crate) graph: Graph,
    pub(crate) schedule: Schedule,
    pub(crate) listeners: HashMap<TypeId, Vec<Box<dyn Listener>>>,
}

impl Reactor {
    pub fn from(graph: Graph) -> Self {
        let schedule = Schedule::from(&graph);

        Self {
            schedule,
            graph,
            listeners: HashMap::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), ReactorError> {
        for root in self.graph.sources().collect::<Vec<_>>() {
            self.notify(root, NodeStatus::Started)?;
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.schedule
            .in_degree
            .copy_from_slice(self.graph.in_degree());
    }

    pub fn listen_for(
        &mut self,
        type_id: TypeId,
        listener: impl Listener,
    ) -> Result<(), ReactorError> {
        let type_exists = self.graph.meta().iter().any(|m| *m.type_id() == type_id);
        if !type_exists {
            return Err(ReactorError::UnknownTypeId);
        }

        self.listeners
            .entry(type_id)
            .or_default()
            .push(Box::new(listener));
        Ok(())
    }

    fn notify(&self, id: NodeId, event: NodeStatus) -> Result<(), ReactorError> {
        let meta = &self.graph.meta()[id];

        let typed_observers = self
            .listeners
            .get(meta.type_id())
            .ok_or(ReactorError::MissingListener)?;

        for obs in typed_observers {
            obs.notify(id, event);
        }

        Ok(())
    }

    pub fn resolve(&mut self, id: NodeId, resolution: Resolution) -> Result<(), ReactorError> {
        let node_statuses = self.schedule.advance(&self.graph, id, resolution);

        for (id, event) in node_statuses {
            self.notify(id, event)?;
        }

        Ok(())
    }

    pub fn node_meta(&self) -> impl Iterator<Item = (NodeId, &Meta)> {
        self.graph.meta().iter().enumerate()
    }
}

pub trait Listener: Send + Sync + 'static {
    fn notify(&self, id: NodeId, event: NodeStatus);
}

impl<F> Listener for F
where
    F: Fn(NodeId, NodeStatus) + Send + Sync + 'static,
{
    fn notify(&self, id: NodeId, event: NodeStatus) {
        self(id, event);
    }
}

impl<O> Listener for Arc<O>
where
    O: Listener + ?Sized,
{
    fn notify(&self, id: NodeId, event: NodeStatus) {
        (**self).notify(id, event);
    }
}
