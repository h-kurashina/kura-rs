## What does this change?

<!-- A new part, a fix to an existing part, or a change to the site / schema. -->

## Checklist

- [ ] `cargo run -p kura-registry -- check` passes
- [ ] For a new part: `registry/<name>.json` has `sample: true` unless the verification and benchmark numbers were produced by running the tests and benchmarks
- [ ] If `crates/kura-schema` changed: ran `cargo run -p kura-registry -- codegen` and committed the generated files
