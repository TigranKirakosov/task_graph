# Task Graph

A declarative, event-driven task scheduler for Rust.

# Planned Features
- [x] fluent and complete [DSL](DSL.md):
    - [x] IDE support
    - [x] compilation error spans
    - [x] compile-time detection of circular dependencies
- [ ] control directives:
    - [ ] [*race*](DSL.md#race)
    - [ ] [*fallback*](DSL.md#fallback)
- [ ] graph visualizer

# Integrations
- [ ] [Bevy plugin](TODO: put link to plugin crate)

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
