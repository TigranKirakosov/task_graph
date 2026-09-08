use ::task_graph::*;

struct Entering;
struct InitScene;
struct Exiting;

fn main() {
    let _graph = task_graph! {
        enter: Entering -> inited: InitScene;
        [inited] -> exit: Exiting;
    };
}
