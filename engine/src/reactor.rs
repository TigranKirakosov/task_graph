use action_orc_core::*;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

pub trait ExternId: std::hash::Hash + std::fmt::Debug + Eq + Clone + Send + Sync + 'static {}
impl<T: std::hash::Hash + std::fmt::Debug + Eq + Clone + Send + Sync + 'static> ExternId for T {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Started,
    Resolved,
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
    pub fn from(graph: Graph, provider: fn(&Meta) -> I) -> Self {
        let mut reactor = Self {
            in_degree: graph.in_degree().to_vec(),
            graph,
            listeners: HashMap::new(),
            extern_to_local: HashMap::new(),
            local_to_extern: Vec::new(),
        };

        for (local_id, meta) in reactor.graph.meta().iter().enumerate() {
            let extern_id = provider(meta);
            reactor.local_to_extern.push(extern_id.clone());
            reactor.extern_to_local.insert(extern_id, local_id);
        }

        reactor
    }

    pub fn init(&mut self) {
        for root in self.graph.sources().collect::<Vec<_>>() {
            self.notify(root, Event::Started);
        }
    }

    pub fn reset(&mut self) {
        self.in_degree.copy_from_slice(self.graph.in_degree());
    }

    pub fn listen_for<T: Marker>(&mut self, listener: impl Listener<I>) {
        let type_id = TypeId::of::<T>();
        self.listeners
            .entry(type_id)
            .or_default()
            .push(Box::new(listener));
    }

    fn notify(&self, id: NodeId, cycle: Event) {
        let meta = &self.graph.meta()[id];

        if let Some(typed_observers) = self.listeners.get(meta.type_id()) {
            let extern_id = &self.local_to_extern[id];
            for obs in typed_observers {
                obs.notify(extern_id.clone(), cycle);
            }
        }
    }

    pub fn resolve(&mut self, id: &I) {
        let Some(&node_id) = self.extern_to_local.get(id) else {
            panic!("Tried to resolve missing id: {id:?}")
        };

        let mut queue = vec![(node_id, Event::Resolved)];

        for &nbr in &self.graph.adj()[node_id] {
            self.in_degree[nbr] = self.in_degree[nbr].saturating_sub(1);
            if self.in_degree[nbr] == 0 {
                queue.push((nbr, Event::Started));
            }
        }

        for (id, cycle) in queue {
            self.notify(id, cycle);
        }
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
    fn notify(&self, id: I, cycle: Event) {
        self(id, cycle);
    }
}

impl<I, O> Listener<I> for Arc<O>
where
    I: ExternId,
    O: Listener<I> + ?Sized,
{
    fn notify(&self, id: I, cycle: Event) {
        (**self).notify(id, cycle);
    }
}
