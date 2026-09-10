use ::action_orc::*;

struct A;
struct B;
struct C;
struct D;

fn main() {
    let _graph = orc! {
        A -> B;
        A -> C;
        A -> D;
    };
}
