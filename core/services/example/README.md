# example

A pump that moves to a level one step at a time, as a cancellable `SetLevel` Job. The smallest complete BlueOS
Rust service, it is never shipped.

```bash
cd core && cargo run -p blueos-example -- --zenoh-endpoint tcp/127.0.0.1:7447
```

Its endpoints are in `ROUTES` of `src/routes.rs`, and `api.lock` records them.
