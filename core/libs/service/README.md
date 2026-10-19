# blueos-service

What every BlueOS Rust service repeats: the command line, logging to the console and to `blueos/v1/<service>/log`,
settings files the Python services share, liveliness, `info`, States, the route table with its Commands and
Queries, Jobs and keyed timers.

A service keeps its own state in one `tokio::select!` loop and uses `zenoh::Session` directly.
`core/services/example` is the smallest complete one, start from it.

The `testing` feature has the helpers for the tests of a service, enable it in `[dev-dependencies]` only.
