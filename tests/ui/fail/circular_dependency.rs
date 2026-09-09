use ::task_graph::*;

struct A;
struct B;

fn main() {
    let _graph = task_graph! {
        a: A -> b: B;
        [b] -> [a];
    };

    let _graph = task_graph! {
        a: A -> B -> [a];
    };

    let combat = task_graph! { A; };

    let _graph = task_graph! {
        A -> #[combat] -> B -> #[combat];
    };
}
