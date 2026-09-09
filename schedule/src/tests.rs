use core::*;
use std::sync::{Arc, Mutex};

use macros::task_graph;

use crate::schedule::*;

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
            #[allow(unused)]
            let $tag = $graph.add_node::<$type>();
        )*
    };
}

#[test]
fn simple_graph() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = Graph::new();
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
        g.sources()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        g.sinks()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn simple_graph_macro() {
    declare_tags!(A, B, C, D, E, F);

    let g = task_graph!(
        A -> B -> C -> D -> E -> F;
    );

    assert_eq!(
        g.sources()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        g.sinks()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn topological_sort() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = Graph::new();
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
        g.sort_ordered()
            .unwrap()
            .into_iter()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A", "B", "C", "E", "D", "F"]
    );
}

#[test]
fn disjoint_sets() {
    declare_tags!(A, B, X, Y);

    let mut g = Graph::new();
    add_nodes!(g,
        a: A, b: B,
        x: X, y: Y
    );

    // Set 1
    g.add_edge(a, b);
    // Set 2
    g.add_edge(x, y);

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // Check upstreams of both sets come before their downstreams
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
    declare_tags!(A, B, C);

    let mut g = Graph::new();
    add_nodes!(g, a: A, b: B, c: C);

    // Loop: A -> B -> C -> A
    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, a);

    assert_eq!(g.sort_ordered().err(), Some(GraphError::CycleDetected));
}

#[test]
fn empty_and_single_node() {
    declare_tags!(A);

    let empty_g = Graph::new();
    assert_eq!(empty_g.sort_ordered().unwrap(), vec![]);

    let mut single_g = Graph::new();
    add_nodes!(single_g, a: A);

    let sorted = single_g.sort_ordered().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(single_g.meta()[sorted[0]].type_name(), "A");
}

#[test]
fn diamond_dependency() {
    declare_tags!(A, B, C, D);

    let mut g = Graph::new();
    add_nodes!(g, a: A, b: B, c: C, d: D);

    g.add_edge(a, b);
    g.add_edge(a, c);
    g.add_edge(b, d);
    g.add_edge(c, d);

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == "B" || sorted[1] == "C");
    assert!(sorted[2] == "B" || sorted[2] == "C");
}

#[test]
fn diamond_dependency_macro() {
    declare_tags!(A, B, C, D);

    let g = task_graph!(
        a: A; d: D;

        [a] -> B -> [d];
        [a] -> C -> [d];
    );

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == "B" || sorted[1] == "C");
    assert!(sorted[2] == "B" || sorted[2] == "C");
}

#[test]
fn lifecycle_hooks() {
    declare_tags!(A, B, C);

    let g = task_graph!(
        A -> B -> C;
    );

    let mut runtime = Schedule::from(g, |meta| meta.type_name());

    let q = Arc::new(Mutex::new(Vec::new()));
    let q_clone = q.clone();
    let q_resolver = Arc::new(move |id, cycle| {
        q_clone.lock().unwrap().push((id, cycle));
    });

    runtime.subscribe::<A>(q_resolver.clone());
    runtime.subscribe::<B>(q_resolver.clone());
    runtime.subscribe::<C>(q_resolver.clone());
    assert_eq!(*q.lock().unwrap(), vec![]);

    runtime.init();
    assert_eq!(*q.lock().unwrap(), vec![("A", Event::Started)]);

    runtime.resolve_task(&"A");
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ("A", Event::Started),
            ("A", Event::Resolved),
            ("B", Event::Started),
        ]
    };

    runtime.resolve_task(&"B");
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ("A", Event::Started),
            ("A", Event::Resolved),
            ("B", Event::Started),
            ("B", Event::Resolved),
            ("C", Event::Started),
        ]
    };

    runtime.resolve_task(&"C");
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ("A", Event::Started),
            ("A", Event::Resolved),
            ("B", Event::Started),
            ("B", Event::Resolved),
            ("C", Event::Started),
            ("C", Event::Resolved),
        ]
    };
}

#[test]
fn nested_schedule_composition() {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    let mut room = Graph::new();
    add_nodes! {
          room,
          enter: Enter, exit: Exit,
    };
    room.add_edge(enter, exit);

    let mut combat = Graph::new();
    add_nodes! {
          combat,
          spawn: SpawnEnemies, fight: Fight,
    };
    combat.add_edge(spawn, fight);

    let mut loot = Graph::new();
    add_nodes! {
          loot,
          roll: RollLoot, pick: PickTreasure,
    };
    loot.add_edge(roll, pick);

    let combat_bounds = room.merge(&combat, vec![enter]);
    let _loot_bounds = room.merge(&loot, combat_bounds.sinks);

    let mut runtime = Schedule::from(room, |meta| meta.type_name());

    let q = Arc::new(Mutex::new(Vec::new()));
    let q_clone = q.clone();
    runtime.subscribe::<Exit>(move |id, event| {
        q_clone.lock().unwrap().push((id, event));
    });

    runtime.init();
    runtime.resolve_task(&"Enter");
    runtime.resolve_task(&"SpawnEnemies");
    runtime.resolve_task(&"Fight");
    runtime.resolve_task(&"RollLoot");

    assert!(q.lock().unwrap().is_empty(), "Exit blocked");

    runtime.resolve_task(&"PickTreasure"); // last task before Exit
    assert_eq!(*q.lock().unwrap(), vec![("Exit", Event::Started)]);
}

#[test]
fn nested_schedule_composition_macro() {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    fn room(a: &Graph, b: &Graph) -> Graph {
        task_graph! {
            Enter -> #[a] -> #[b] -> Exit;
        }
    }

    let combat = task_graph!(
        SpawnEnemies -> Fight;
    );

    let loot = task_graph!(
        RollLoot -> PickTreasure;
    );

    let composed_room = room(&combat, &loot);

    let mut runtime = Schedule::from(composed_room, |meta| meta.type_name());

    let q = Arc::new(Mutex::new(Vec::new()));
    let q_clone = q.clone();
    runtime.subscribe::<Exit>(move |id, event| {
        q_clone.lock().unwrap().push((id, event));
    });

    runtime.init();
    runtime.resolve_task(&"Enter");
    runtime.resolve_task(&"SpawnEnemies");
    runtime.resolve_task(&"Fight");
    runtime.resolve_task(&"RollLoot");

    assert!(q.lock().unwrap().is_empty(), "Exit blocked");

    runtime.resolve_task(&"PickTreasure"); // last task before Exit
    assert_eq!(*q.lock().unwrap(), vec![("Exit", Event::Started)]);
}

/// G: (a | b) -> c
/// H: x -> y
#[test]
fn merge_into_parallel_set() {
    declare_tags!(A, B, C);
    declare_tags!(X, Y);

    // (a | b) -> c
    let mut g = Graph::new();
    add_nodes! {
          g,
          a: A, b: B, c: C,
    };
    g.add_edge(a, c);
    g.add_edge(b, c);

    // x -> y
    let mut h = Graph::new();
    add_nodes!(h, x: X, y: Y);
    h.add_edge(x, y);

    // (a | b) -> x -> y -> c
    let _h_bounds = g.merge(&h, vec![a, b]);

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}

/// - G: `(a | b) -> #[sub] -> c`
/// - H: `x -> y`
/// - Combined: `(a | b) -> x -> y -> c`
#[test]
fn merge_into_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let h = task_graph!(
        X -> Y;
    );

    // (a | b) -> #[sub] -> c
    let g = task_graph!(
        (A | B) -> #[h] -> C;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}

/// - G: `Enter -> ( A | #[sub] ) -> Exit`
/// - #[sub]: `X -> Y`
/// - Combined: `Enter -> (A | X -> Y) -> Exit`
#[test]
fn embed_graph_inside_parallel_group_macro() {
    declare_tags!(X, Y);
    declare_tags!(Enter, A, Exit);

    let sub = task_graph!(
        X -> Y;
    );

    let g = task_graph!(
        Enter -> ( A | #[sub] ) -> Exit;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order[0], "Enter");
    assert!(order[1] == "A" || order[1] == "X");
    assert!(order[2] == "A" || order[2] == "X" || order[2] == "Y");
    assert_eq!(*order.last().unwrap(), "Exit");
}

/// - #[sub]: `X -> Y`
/// - G: `#[sub] -> (A | B) -> C`
/// - Combined: `X -> Y -> (A | B) -> C`
#[test]
fn embed_graph_fan_out_to_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let sub = task_graph!(
        X -> Y;
    );

    let g = task_graph!(
        #[sub] -> (A | B) -> C;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["X", "Y", "A", "B", "C"]);
}

#[test]
fn standalone_embedding_macro() {
    declare_tags!(A, B);
    let sub = task_graph!(A -> B;);

    let g = task_graph!(
        #[sub];
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B"]);
}
