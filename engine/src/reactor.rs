use action_orc_core::*;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

pub trait ExternId: std::hash::Hash + std::fmt::Debug + Eq + Clone + Send + Sync + 'static {}
impl<T: std::hash::Hash + std::fmt::Debug + Eq + Clone + Send + Sync + 'static> ExternId for T {}

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
    ExternIdMismatch,
    UnknownTypeId,
}

pub struct Reactor<I: ExternId> {
    pub(crate) graph: Graph,
    pub(crate) in_degree: Vec<usize>,
    pub(crate) listeners: HashMap<TypeId, Vec<Box<dyn Listener<I>>>>,
    pub(crate) extern_to_local: HashMap<I, NodeId>,
    pub(crate) local_to_extern: Vec<I>,
}

impl<I> Reactor<I>
where
    I: ExternId,
{
    pub fn from(graph: Graph, mut id_provider: impl IdProvider<I>) -> Self {
        let mut extern_to_local = HashMap::new();
        let mut local_to_extern = Vec::new();

        for (local_id, meta) in graph.meta().iter().enumerate() {
            let extern_id = id_provider.provide(meta);
            local_to_extern.push(extern_id.clone());
            extern_to_local.insert(extern_id, local_id);
        }

        Self {
            in_degree: graph.in_degree().to_vec(),
            graph,
            listeners: HashMap::new(),
            extern_to_local,
            local_to_extern,
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
        listener: impl Listener<I>,
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

        let extern_id = &self.local_to_extern[id];
        for obs in typed_observers {
            obs.notify(extern_id.clone(), event);
        }

        Ok(())
    }

    pub fn resolve(&mut self, id: &I, resolution: Resolution) -> Result<(), ReactorError> {
        let node_id = *self
            .extern_to_local
            .get(id)
            .ok_or(ReactorError::ExternIdMismatch)?;

        let mut queue = vec![(node_id, Event::Resolved(resolution))];

        for &nbr in &self.graph.adj()[node_id] {
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
}

pub trait Listener<I: ExternId>: Send + Sync + 'static {
    fn notify(&self, id: I, event: Event);
}

impl<I, F> Listener<I> for F
where
    I: ExternId,
    F: Fn(I, Event) + Send + Sync + 'static,
{
    fn notify(&self, id: I, event: Event) {
        self(id, event);
    }
}

impl<I, O> Listener<I> for Arc<O>
where
    I: ExternId,
    O: Listener<I> + ?Sized,
{
    fn notify(&self, id: I, event: Event) {
        (**self).notify(id, event);
    }
}

pub trait IdProvider<I: ExternId> {
    fn provide(&mut self, meta: &Meta) -> I;
}

impl<I, F> IdProvider<I> for F
where
    I: ExternId,
    F: FnMut(&Meta) -> I,
{
    fn provide(&mut self, meta: &Meta) -> I {
        self(meta)
    }
}
