use syn::{Ident, Type};

use super::format_type;
use super::parser::SpanInfo;

#[derive(Debug)]
pub(super) struct TaskGraphAst {
    pub(super) graphs: Vec<Graph>,
}

/// `A -> (B | [in]) -> C`
#[derive(Debug, PartialEq)]
pub(super) struct Graph {
    // A
    pub(super) entry: NodeExpr,
    // B, [in], C
    pub(super) conns: Vec<NodeExpr>,
    pub(super) span_info: SpanInfo,
}

#[derive(Debug, PartialEq)]
pub(super) enum NodeExpr {
    /// A local variable declaration
    Declaration(Task),

    /// A bound identifier reference, e.g. [in]
    Binding(Ident),

    /// (A | B | C) or (A, B, C) or (A -> B -> C)
    Group(GroupBlock),
}

/// Local task declaration signature
/// Either full match `var: typ` (`a: TaskA`) or type only (`TaskA`)
#[derive(PartialEq)]
pub(super) struct Task {
    pub(super) var: Option<Ident>,
    pub(super) typ: Type,
}

#[derive(Debug, PartialEq)]
pub(super) struct GroupBlock {
    pub(super) mode: SchedulingMode,
    pub(super) graphs: Vec<Graph>,
    pub(super) span_info: SpanInfo,
}

#[derive(Debug, PartialEq)]
pub(super) enum SchedulingMode {
    /// (A, B) or (A -> B)
    Sequence,
    /// (A | B | C)
    Parallel,
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_string = format_type(&self.typ);

        f.debug_struct("Task")
            .field("var", &self.var.clone().map(|var| var.to_string()))
            .field("typ", &type_string)
            .finish()
    }
}
