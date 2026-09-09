use ::task_graph::*;

struct Enter;
struct Exit;
struct TaskA;
struct TaskB;
struct TaskC;

fn combat_fn() -> Graph {
    task_graph! {
        TaskA -> TaskB;
    }
}

fn loot_fn() -> Graph {
    task_graph! {
        TaskC;
    }
}

fn main() {
    let (combat, loot) = (combat_fn(), loot_fn());

    let _linear_composed = task_graph! {
        Enter -> #[combat] -> #[loot] -> Exit;
    };

    let _parallel_composed = task_graph! {
        Enter -> ( TaskA | #[combat] | TaskB ) -> Exit;
    };

    let _start_embedded = task_graph! {
        #[combat] -> Exit;
    };
}
