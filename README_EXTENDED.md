# Air — Extended README (Developer-focused)

This companion README provides quick developer-focused reference material and pointers to in-repo documentation.

Key docs

- `docs/API.md` — gRPC API reference and examples.
- `docs/CONFIGURATION.md` — configuration keys and environment examples.
- `docs/CODE_REFERENCE.md` — public crate API and usage patterns.
- `docs/DEVELOPER_GUIDE.md` — contribution, testing, and performance tips.
- `CONTRIBUTING.md` — how to contribute.

Quickstart

1. Build: `cargo build --manifest-path Element/Air/Cargo.toml --release`
2. Test: `cargo test --manifest-path Element/Air/Cargo.toml`
3. Run locally: `./target/release/air --config ./examples/config/local.toml`

Production notes

- Use `/etc/air/config.toml` for system installs and enable TLS. See `docs/CONFIGURATION.md`.
- Use a supervising service (systemd) and monitor logs and `GetMetrics` for health.

Contact

For issues, open an issue at https://github.com/CodeEditorLand/Air/issues or contact the maintainers listed in the main README.
