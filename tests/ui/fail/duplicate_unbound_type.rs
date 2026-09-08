use ::task_graph::*;

struct A;
struct B;
struct C;
struct D;

fn main() {
    let _graph = task_graph! {
        A -> B;
        A -> C;
        A -> D;
    };
}
