Air Tests
===========

How to run tests locally:

- Unit & integration tests:

```bash
cd Application/CodeEditorLand/Land/Element/Air
./scripts/test.sh
```

- To generate coverage (requires `cargo-tarpaulin`):

```bash
cargo install cargo-tarpaulin
./scripts/test.sh
```

Notes:
- Unit tests reuse the existing `tests/mock_services.rs` via `tests/common/mod.rs`.
- Integration and performance tests are located in `tests/` and can be run via `cargo test`.
