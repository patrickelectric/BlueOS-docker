# blueos-idl

The BlueOS interface types, generated from the ROS 2 interface files in `interfaces/`, and their CDR encoding.

After changing an interface, regenerate the types and the lock, and commit both with the change:

```bash
cd core && cargo run -p blueos-idl-codegen
```

CI runs the generator again and fails if `src/generated.rs` or `api.lock` differ.

Plain CDR has no schema evolution, and the readers already deployed fill nothing in. So a message never changes
once it is on master, an added field included: the change is a new message type. `api.lock` may only gain lines,
CI fails when one of master is gone.
