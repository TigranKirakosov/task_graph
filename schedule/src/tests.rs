use std::sync::{Arc, Mutex};

use super::builder::*;
use super::schedule::*;

macro_rules! declare_tags {
    ($($type:ident),* $(,)?) => {
        $(
            struct $type;
        )*
    };
}

macro_rules! add_tasks {
    ($graph:expr, $($tag:ident : $type:ty),* $(,)?) => {
        $(
            let $tag = $graph.add_task::<$type>();
        )*
    };
}

#[test]
fn simple_graph() {
    let mut schedule = Schedule::<char, Building>::new();
    declare_tags!(A, B, C, D, E, F);
    add_tasks!(
        schedule,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    schedule.add_dep(a, b);
    schedule.add_dep(b, c);
    schedule.add_dep(c, d);
    schedule.add_dep(d, e);
    schedule.add_dep(e, f);

    assert_eq!(
        schedule
            .roots()
            .map(|id| schedule.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        schedule
            .leaves()
            .map(|id| schedule.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn topological_sort() {
    let mut g = Schedule::<char, Building>::new();
    declare_tags!(A, B, C, D, E, F);
    add_tasks!(
        g,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    g.add_dep(a, b);
    g.add_dep(a, c);
    g.add_dep(a, e);
    g.add_dep(b, d);
    g.add_dep(c, d);
    g.add_dep(e, f);

    assert_eq!(
        g.sort_ordered()
            .unwrap()
            .into_iter()
            .map(|id| g.node_meta[id].type_name)
            .collect::<Vec<_>>(),
        vec!["A", "B", "C", "E", "D", "F"]
    );
}

#[test]
fn disjoint_sets() {
    let mut g = Schedule::<char, Building>::new();
    declare_tags!(A, B, X, Y);
    add_tasks!(g,
        a: A, b: B,
        x: X, y: Y
    );

    // Set 1
    g.add_dep(a, b);
    // Set 2
    g.add_dep(x, y);

    let sorted: Vec<_> = g
        .sort_ordered()
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
    let mut g = Schedule::<char, Building>::new();
    declare_tags!(A, B, C);
    add_tasks!(g, a: A, b: B, c: C);

    // Loop: A -> B -> C -> A
    g.add_dep(a, b);
    g.add_dep(b, c);
    g.add_dep(c, a);

    assert_eq!(g.sort_ordered().err(), Some(GraphError::CycleDetected));
}

#[test]
fn empty_and_single_node() {
    let empty_g = Schedule::<char, Building>::new();
    assert_eq!(empty_g.sort_ordered().unwrap(), vec![]);

    let mut single_g = Schedule::<char, Building>::new();
    declare_tags!(A);
    add_tasks!(single_g, a: A);

    let sorted = single_g.sort_ordered().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(single_g.node_meta[sorted[0]].type_name, "A");
}

#[test]
fn diamond_dependency() {
    let mut g = Schedule::<char, Building>::new();

    declare_tags!(A, B, C, D);
    add_tasks!(g, a: A, b: B, c: C, d: D);

    g.add_dep(a, b);
    g.add_dep(a, c);
    g.add_dep(b, d);
    g.add_dep(c, d);

    let sorted: Vec<_> = g
        .sort_ordered()
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

#[test]
fn lifecycle_hooks() {
    let mut g = Schedule::<char, Building>::new();
    declare_tags!(A, B, C);
    add_tasks!(
        g,
        a: A, b: B, c: C,
    );

    g.add_dep(a, b);
    g.add_dep(b, c);

    let mut runtime = g.build(|meta| match meta.type_name {
        "A" => 'A',
        "B" => 'B',
        "C" => 'C',
        _ => unreachable!(),
    });

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
    assert_eq!(*q.lock().unwrap(), vec![('A', Event::Started)]);

    runtime.resolve_task(&'A');
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ('A', Event::Started),
            ('A', Event::Resolved),
            ('B', Event::Started),
        ]
    };

    runtime.resolve_task(&'B');
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ('A', Event::Started),
            ('A', Event::Resolved),
            ('B', Event::Started),
            ('B', Event::Resolved),
            ('C', Event::Started),
        ]
    };

    runtime.resolve_task(&'C');
    assert_eq! {
        *q.lock().unwrap(),
        vec![
            ('A', Event::Started),
            ('A', Event::Resolved),
            ('B', Event::Started),
            ('B', Event::Resolved),
            ('C', Event::Started),
            ('C', Event::Resolved),
        ]
    };
}

#[test]
fn nested_schedule_composition() {
    let mut room = Schedule::<char, Building>::new();
    declare_tags!(Enter, Exit);
    add_tasks! {
          room,
          enter: Enter, exit: Exit,
    };
    room.add_dep(enter, exit);

    let mut combat = Schedule::<char, Building>::new();
    declare_tags!(Spawn, Fight);
    add_tasks! {
          combat,
          spawn: Spawn, fight: Fight,
    };
    combat.add_dep(spawn, fight);

    let mut loot_room = Schedule::<char, Building>::new();
    declare_tags!(RollLoot, PickTreasure);
    add_tasks! {
          loot_room,
          roll: RollLoot, pick: PickTreasure,
    };
    loot_room.add_dep(roll, pick);

    let combat_leaves = room.merge(combat, vec![enter]);
    let _loot_room_leaves = room.merge(loot_room, combat_leaves);

    let mut runtime = room.build(|meta| match meta.type_name {
        "Enter" => 'E',
        "Spawn" => 'S',
        "Fight" => 'F',
        "RollLoot" => 'L',
        "PickTreasure" => 'P',
        "Exit" => 'X',
        _ => unreachable!(),
    });

    let q = Arc::new(Mutex::new(Vec::new()));
    let q_clone = q.clone();
    runtime.subscribe::<Exit>(move |id, event| {
        q_clone.lock().unwrap().push((id, event));
    });

    runtime.init();
    runtime.resolve_task(&'E');
    runtime.resolve_task(&'S');
    runtime.resolve_task(&'F');
    runtime.resolve_task(&'L');

    assert!(q.lock().unwrap().is_empty(), "Exit blocked");

    runtime.resolve_task(&'P'); // last task before Exit
    assert_eq!(*q.lock().unwrap(), vec![('X', Event::Started)]);
}

#[test]
fn merge_into_parallel_set() {
    // Graph: (a | b) -> c
    let mut g = Schedule::<char, Building>::new();
    declare_tags!(A, B, C);
    add_tasks! {
          g,
          a: A, b: B, c: C,
    };
    g.add_dep(a, c);
    g.add_dep(b, c);

    // Graph: x -> y
    let mut h = Schedule::<char, Building>::new();
    declare_tags!(X, Y);
    add_tasks!(h, x: X, y: Y);
    h.add_dep(x, y);

    // Graph: (a | b) -> x -> y -> c
    let _h_leaves = g.merge(h, vec![a, b]);

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.node_meta[id].type_name)
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}
