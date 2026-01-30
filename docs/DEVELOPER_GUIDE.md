# Developer Guide

This guide covers contribution, testing, and performance tips for working on Air.

Contribution workflow

- Fork and create feature branches `feat/<short-desc>` or `fix/<short-desc>`.
- Keep PRs small and focused; include tests and documentation updates.
- Write clear commit messages and update `CHANGELOG.md` for user-visible changes.

Coding style

- Rust code follows `rustfmt` defaults and `clippy` lints. Run:

```bash
cargo fmt --manifest-path Element/Air/Cargo.toml
cargo clippy --manifest-path Element/Air/Cargo.toml -- -D warnings
```

Testing

- Unit tests: `cargo test --manifest-path Element/Air/Cargo.toml`
- Integration tests: run `cargo test --manifest-path Element/Air/Cargo.toml --test integration` or use the included `tests/` helpers.
- End-to-end: start a local `air` instance and run `examples/` or use the mock services under `tests/mock_services.rs`.

Performance and profiling

- Use `cargo bench` for micro-benchmarks.
- For CPU/profile-level analysis, build with `--release` and use `perf` or `Instruments` on macOS.
- For memory analysis, use `valgrind`/`massif` or Rust-specific tools like `heaptrack`.

CI

- Ensure tests and linters run in CI; include a job for building docs (`cargo doc`) and running `cargo clippy`.

Local dev tips

- Use `RUST_LOG=debug` to get verbose logs.
- Use small `examples/config` files to iterate quickly.

If you'd like, I can add CI workflow files with these steps.
