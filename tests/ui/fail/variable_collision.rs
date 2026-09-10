use ::action_orc::*;

struct A;
struct B;

fn main() {
    let _graph = orc! {
        a: A -> b: B;

        a: B;
    };
}
