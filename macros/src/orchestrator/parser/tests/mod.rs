use std::str::FromStr;

use super::*;

#[test]
fn atomic_single_task() {
    let tokens = tokenize("a: TaskA;");
    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn complex_graph() {
    let tokens = tokenize(
        "
         enter: Entering -> inited: InitScene;

         [inited] -> (A | B | C) -> exit: Exiting;
     ",
    );

    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

fn tokenize(i: &str) -> TokenStream2 {
    TokenStream2::from_str(i).unwrap()
}
