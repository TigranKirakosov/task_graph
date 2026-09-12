mod reactor;
mod schedule;

pub use reactor::*;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    Started,
    Resolved(Resolution),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    Finished,
}
