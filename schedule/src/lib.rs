mod graph;
mod meta;
mod schedule;

#[cfg(test)]
mod tests;

pub trait TaskMarker: 'static {}
impl<T: 'static> TaskMarker for T {}

pub type InternId = usize;

pub trait ExternId: std::hash::Hash + Eq + Clone + Send + Sync + 'static {}
impl<T: std::hash::Hash + Eq + Clone + Send + Sync + 'static> ExternId for T {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Started,
    Resolved,
}
