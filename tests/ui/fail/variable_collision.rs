use ::task_graph::*;

struct A;
struct B;

fn main() {
    let _graph = task_graph! {
        a: A -> b: B;

        a: B;
    };
}
