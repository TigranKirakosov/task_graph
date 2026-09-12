use action_orc_core::*;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Started,
    Resolved(Resolution),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    Finished,
    Cancelled,
}

#[derive(Debug)]
pub enum ReactorError {
    MissingListener,
    UnknownTypeId,
}

pub struct Reactor {
    pub(crate) graph: Graph,
    pub(crate) in_degree: Vec<usize>,
    pub(crate) listeners: HashMap<TypeId, Vec<Box<dyn Listener>>>,
}

impl Reactor {
    pub fn from(graph: Graph) -> Self {
        Self {
            in_degree: graph.in_degree().to_vec(),
            graph,
            listeners: HashMap::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), ReactorError> {
        for root in self.graph.sources().collect::<Vec<_>>() {
            self.notify(root, Event::Started)?;
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.in_degree.copy_from_slice(self.graph.in_degree());
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

    fn notify(&self, id: NodeId, event: Event) -> Result<(), ReactorError> {
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
        let mut queue = vec![(id, Event::Resolved(resolution))];

        for &nbr in &self.graph.adj()[id] {
            self.in_degree[nbr] = self.in_degree[nbr].saturating_sub(1);
            if self.in_degree[nbr] == 0 {
                queue.push((nbr, Event::Started));
            }
        }

        for (id, event) in queue {
            self.notify(id, event)?;
        }

        Ok(())
    }

    pub fn node_meta(&self) -> impl Iterator<Item = (NodeId, &Meta)> {
        self.graph.meta().iter().enumerate()
    }
}

pub trait Listener: Send + Sync + 'static {
    fn notify(&self, id: NodeId, event: Event);
}

impl<F> Listener for F
where
    F: Fn(NodeId, Event) + Send + Sync + 'static,
{
    fn notify(&self, id: NodeId, event: Event) {
        self(id, event);
    }
}

impl<O> Listener for Arc<O>
where
    O: Listener + ?Sized,
{
    fn notify(&self, id: NodeId, event: Event) {
        (**self).notify(id, event);
    }
}
