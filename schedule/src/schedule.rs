use std::any::TypeId;

use crate::{lifecycle::Listener, meta::Meta};

#[derive(Debug, PartialEq)]
pub(crate) enum GraphError {
    CycleDetected,
}

pub(crate) trait TaskMarker: 'static {}
impl<T: 'static> TaskMarker for T {}

pub(crate) type InternId = usize;

pub(crate) trait ExternId: std::hash::Hash + Eq + Clone + Send + Sync + 'static {}
impl<T: std::hash::Hash + Eq + Clone + Send + Sync + 'static> ExternId for T {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Event {
    Started,
    Resolved,
}

pub(crate) struct Schedule<I, S>
where
    I: ExternId,
    S: 'static,
{
    pub(crate) listeners: std::collections::HashMap<TypeId, Vec<Box<dyn Listener<I>>>>,

    /// Empty before transition to [Runtime]
    pub(crate) extern_to_local: std::collections::HashMap<I, InternId>,
    /// Empty before transition to [Runtime]
    pub(crate) local_to_extern: Vec<I>,

    pub(crate) in_deg: Vec<usize>,
    pub(crate) adj: Vec<Vec<InternId>>,
    pub(crate) node_meta: Vec<Meta>,
    pub(crate) _marker: std::marker::PhantomData<S>,
}
