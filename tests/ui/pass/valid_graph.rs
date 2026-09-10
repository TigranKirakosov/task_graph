use ::action_orc::*;

struct Entering;
struct InitScene;
struct Exiting;

fn main() {
    let _graph = orc! {
        enter: Entering -> inited: InitScene;
        [inited] -> exit: Exiting;
    };
}
