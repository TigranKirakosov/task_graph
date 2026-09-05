```rust
// DSL schedule composition concept (WIP)

/// Holds nodes and maps external [I] id to internal usize node identifier
/// Holds an [R] root type marker used in graph composition
struct TaskGraph<I, R> where I: Hashable, R: RootTask { ... }

type IdProvider<I> = fn() -> I;
impl<I, R> TaskGraph<I, R> {
    fn build(self, provider: IdProvider<I>) -> RuntimeSchedule<I> { ... }
}

// Define parametrized dungeon flow:
// - handle entering room
// - start 2 parallel tasks
// - start generic encounter
// - handle exiting room
fn dungeon_room<I, R>(encounter: TaskGraph<I, R>) -> TaskGraph<I, Entering> {
    task_graph! {
        Entering -> (LoreNarration | Animations) -> [encounter] -> Exiting;
    }
}

// Define custom encounters
fn goblins<I>() -> TaskGraph<I, SpawnGoblins> {
    task_graph! {
        SpawnGoblins -> ResolveCombat -> RollReward;
    }
}

fn merchant<I>() -> TaskGraph<I, OpenShopUI> {
    task_graph! {
        OpenShopUI -> AwaitPlayerTransaction;
    }
};

// Compose
fn main() {
    let mut id = 0;
    let mut counter = || -> usize {
        id += 1;
        id
    }
    
    let goblin_encounter = goblins();
    let merchant_encounter = merchant();
    
    let goblin_room = dungeon_room(goblin_encounter);
    let merchant_room = dungeon_room(merchant_encounter);

    let goblin_room_schedule = goblin_room.build(counter);
    let merchant_room_schedule = goblin_room.build(counter);
}
```
