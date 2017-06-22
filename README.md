# scheduled-executor

A simple function scheduler.

## The library

This library provides a series of utilities for scheduling and executing tasks.
Tasks can be executed at fixed interval or at fixed rates, and can be executed in the main
executor thread or using a thread pool.

### Executors

- `CoreExecutor`: schedule and execute tasks on a single thread, ideal for short running tasks.
- `ThreadPoolExecutor`: schedule and execute tasks on a thread pool. Can be used for long
running tasks.

### Documentation

- [Current master branch](https://fede1024.github.io/rust-scheduled-executor/)

### Examples

Scheduling periodic task is very simple. Here is an example using a thread pool:

```rust
// Starts a new thread-pool based executor with 4 threads
let executor = ThreadPoolExecutor::new(4)?;

executor.schedule_fixed_rate(
    Duration::from_secs(2),  // Wait 2 seconds before scheduling the first task
    Duration::from_secs(5),  // and schedule every following task at 5 seconds intervals
    |remote| {
        // Code to be scheduled. The code will run on one of the threads in the thread pool.
        // The `remote` handle can be used to schedule additional work on the event loop,
        // if needed.
    },
);
```
