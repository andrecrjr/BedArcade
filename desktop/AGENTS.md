# Desktop App Instructions

- Follow the repository-level instructions in `../AGENTS.md`.
- Keep this Rust desktop app independent from the browser extension.
- Initial targets are Windows and Linux.
- Use RED → GREEN for behavior: add a failing test before implementing the behavior.
- Check platform support before selecting gamepad, accessibility, or input simulation dependencies.
- Treat item navigation and mouse control as separate modes; do not promise every app exposes an accessibility tree.
- Run `cargo fmt` and `cargo clippy` for completed Rust changes.
