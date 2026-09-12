mod reactor;
mod schedule;

pub use reactor::*;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    /// Once a node got this status, it **obliged** to eventually call [Reactor::resolve]
    /// in order drive schedule advancement. Otherwise, whole **engine will stall**.
    Started,
    Resolved(Resolution),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    Finished,
}
