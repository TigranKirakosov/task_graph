# Task Graph

A declarative, event-driven DAG workflow orchestration engine.

# Planned Features
- fluent proc-macro [DSL](DSL.md) with fully preserved Rust intellisense which does:
    - provide error spans
    - maintain DAG property
- declare and compose parametrized schedules at compile time
- generic lifecycle hooks
- graph visualizer
- Bevy plugin integration

## Syntax Overview

- Declare a task with handle `task: Marker` or anonymously `Marker`
- Declare a dependency between tasks: `lhs -> rhs` (reading, **lhs** blocks **rhs**)
- Declare a jointed graph set:
    - `a` and `x` are jointed by `[b] -> y`
    - `a` and `m` are jointed by `[in] -> [m]`
- Bind already declared tasks: `[b] -> y`
- Schedule grouped tasks to run either in *Sequence* or *Parallel* at runtime:
    - declare task sequence: `a -> (b, c)`
    - declare parallel tasks: `x -> (y | z)`

```rust
fn example_schedule(in) {
    task_graph! {
        TaskA -> (
            b: TaskB,
            TaskC,
            [in]
        );

        m: TaskM;
    
        TaskX -> (
            [b] -> TaskY
            |
            [in] -> [m] // `m` wont start until `in` (input graph) is resolved
        );
    }
};
```

## License

Copyright © 2026 Tigran Kirakosov.

This project is dual-licensed under either:

* **MIT License** ([LICENSE-MIT](LICENSE-MIT))
* **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
