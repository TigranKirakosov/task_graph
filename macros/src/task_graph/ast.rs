use syn::{Ident, Type};

pub(super) struct TaskGraphAst {
    pub(super) graphs: Vec<Graph>,
}

/// `A -> (B | [in]) -> C`
pub(super) struct Graph {
    // A
    pub(super) entry: NodeExpr,
    // B, [in], C
    pub(super) conns: Vec<NodeExpr>,
}

pub(super) enum NodeExpr {
    /// A local variable declaration
    Declaration(Task),

    /// A bound identifier reference, e.g. [in]
    Binding(Ident),

    /// (A | B | C) or (A, B, C) or (A -> B -> C)
    Group(GroupBlock),
}

/// Local task declaration signature
/// Either full match `var: typ` (`a: TaskA`) or type only `typ` (`TaskA`)
pub(super) struct Task {
    pub(super) var: Option<Ident>,
    pub(super) typ: Type,
}

pub(super) struct GroupBlock {
    pub(super) mode: SchedulingMode,
    pub(super) graphs: Vec<Graph>,
}

pub(super) enum SchedulingMode {
    /// (A, B) or (A -> B)
    Sequence,
    /// (A | B | C)
    Parallel,
}
