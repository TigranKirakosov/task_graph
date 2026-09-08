## DSL Concept
```rust
// Define parametrized dungeon flow:
// 1. handle entering room
// 2. start 2 parallel tasks
// 3. start generic encounter
// 4. handle exiting room
fn dungeon_room(encounter: Graph) -> Graph {
    task_graph! {
        // you can use both generic and pure labels
        Entering<Armed> -> (LoreNarration | Animations) -> [encounter] -> Exiting<UnArmed>;
    }
}

// Define custom encounters
fn goblins() -> Graph {
    task_graph! {
        SpawnGoblins -> ResolveCombat -> RollReward;
    }
}

fn merchant() -> Graph {
    task_graph! {
        OpenShopUI -> AwaitPlayerTransaction;
    }
}

// Compose
fn main() {
    let goblin_encounter = goblins();
    let merchant_encounter = merchant();

    let goblin_room = dungeon_room(goblin_encounter);
    let merchant_room = dungeon_room(merchant_encounter);

    let goblin_room_schedule = Schedule::from(goblin_room, |_| /* map unique ids*/ "your_hashable_id");
    let merchant_room_schedule = Schedule::from(merchant_room, |_| /* map unique ids*/ "your_hashable_id");

    goblin_room_schedule.subscribe::<ResolveCombat>(|id, event| match event {
        Event::Started => { ... }, // receive control and eventually resolve a task
        Event::Resolved => { ... }
    });

    goblin_room_schedule.init();
}
```

# Directives

## race
If `a` finishes first, `b` and `c` are discarded while `d` gets started:
`race(a | b | c) -> d`

## fallback
Mark fallback node from where to recover when a downstream has failed; provide retry count:
`fallback(start, retry_count) -> (a | b | c) -> d`
