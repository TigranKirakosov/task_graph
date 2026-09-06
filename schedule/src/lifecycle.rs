use std::{any::TypeId, sync::Arc};

use crate::meta::Meta;
use crate::schedule::{Event, ExternId, InternId, Schedule, TaskMarker};

pub(crate) trait Listener<I: ExternId>: Send + Sync + 'static {
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

pub(crate) struct Running;

impl<I> Schedule<I, Running>
where
    I: ExternId,
{
    pub(crate) fn subscribe<T: TaskMarker>(&mut self, listener: impl Listener<I>) {
        let type_id = TypeId::of::<T>();
        self.listeners
            .entry(type_id)
            .or_default()
            .push(Box::new(listener));
    }

    fn notify(&self, id: InternId, cycle: Event) {
        let Meta { type_id, .. } = &self.node_meta[id];

        if let Some(typed_observers) = self.listeners.get(type_id) {
            let extern_id = &self.local_to_extern[id];
            for obs in typed_observers {
                obs.notify(extern_id.clone(), cycle);
            }
        }
    }

    pub(crate) fn init(&mut self) {
        for root in self.roots().collect::<Vec<_>>() {
            self.notify(root, Event::Started);
        }
    }

    pub(crate) fn resolve_task(&mut self, id: &I) {
        let &node_id = self.extern_to_local.get(id).unwrap();
        let mut queue = vec![(node_id, Event::Resolved)];

        for &nbr in &self.adj[node_id] {
            self.in_deg[nbr] = self.in_deg[nbr].saturating_sub(1);
            if self.in_deg[nbr] == 0 {
                queue.push((nbr, Event::Started));
            }
        }

        for (id, cycle) in queue {
            self.notify(id, cycle);
        }
    }
}
