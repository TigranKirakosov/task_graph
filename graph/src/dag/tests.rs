use super::*;

macro_rules! declare_tags {
    ($($type:ident),* $(,)?) => {
        $(
            struct $type;
        )*
    };
}

macro_rules! add_nodes {
    ($graph:expr, $($tag:ident : $type:ty),* $(,)?) => {
        $(
            let $tag = $graph.add_node::<$type>();
        )*
    };
}

#[test]
fn simple_graph() {
    let mut g = DAG::<char>::default();
    declare_tags!(A, B, C, D, E, F);
    add_nodes!(
        g,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, d);
    g.add_edge(d, e);
    g.add_edge(e, f);

    assert_eq!(
        g.roots()
            .map(|id| g.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        g.leaves()
            .map(|id| g.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn topological_sort() {
    let mut g = DAG::<char>::default();
    declare_tags!(A, B, C, D, E, F);
    add_nodes!(
        g,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    g.add_edge(a, b);
    g.add_edge(a, c);
    g.add_edge(a, e);
    g.add_edge(b, d);
    g.add_edge(c, d);
    g.add_edge(e, f);

    assert_eq!(
        g.topological_sort()
            .unwrap()
            .into_iter()
            .map(|id| g.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["A", "B", "C", "E", "D", "F"]
    );
}

#[test]
fn disjoint_sets() {
    let mut g = DAG::<char>::default();
    declare_tags!(A, B, X, Y);
    add_nodes!(g,
        a: A, b: B,
        x: X, y: Y
    );

    // Set 1
    g.add_edge(a, b);
    // Set 2
    g.add_edge(x, y);

    let sorted: Vec<_> = g
        .topological_sort()
        .unwrap()
        .into_iter()
        .map(|id| g.node_meta[id].type_name)
        .collect();

    // Check roots of both sets come before their downstreams
    assert!(
        sorted.iter().position(|&name| name == "A").unwrap()
            < sorted.iter().position(|&name| name == "B").unwrap()
    );
    assert!(
        sorted.iter().position(|&name| name == "X").unwrap()
            < sorted.iter().position(|&name| name == "Y").unwrap()
    );
}

#[test]
fn cyclic_graph_returns_err() {
    let mut g = DAG::<char>::default();
    declare_tags!(A, B, C);
    add_nodes!(g, a: A, b: B, c: C);

    // Loop: A -> B -> C -> A
    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, a);

    assert_eq!(
        g.topological_sort().err(),
        Some(SortingError::CycleDetected)
    );
}

#[test]
fn empty_and_single_node() {
    let empty_g = DAG::<char>::default();
    assert_eq!(empty_g.topological_sort().unwrap(), vec![]);

    let mut single_g = DAG::<char>::default();
    declare_tags!(A);
    add_nodes!(single_g, a: A);

    let sorted = single_g.topological_sort().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(single_g.node_meta[sorted[0]].type_name, "A");
}

#[test]
fn diamond_dependency() {
    let mut g = DAG::<char>::default();
    declare_tags!(A, B, C, D);
    add_nodes!(g, a: A, b: B, c: C, d: D);

    g.add_edge(a, b);
    g.add_edge(a, c);
    g.add_edge(b, d);
    g.add_edge(c, d);

    let sorted: Vec<_> = g
        .topological_sort()
        .unwrap()
        .into_iter()
        .map(|id| g.node_meta[id].type_name)
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == "B" || sorted[1] == "C");
    assert!(sorted[2] == "B" || sorted[2] == "C");
}
