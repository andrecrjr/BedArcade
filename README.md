# Gamepad Desktop

Use a gamepad to control the active desktop window, move the pointer, navigate accessible controls, and type with an on-screen keyboard. The app supports Windows and Linux; the browser extension is a separate project under [`extension/`](extension/README.md).

## Start

For local development, install Rust and the desktop dependencies described in the [system overview](desktop/docs/system-overview.md#como-iniciar-no-linux), then run:

```bash
make desktop-run
```

The Windows release ZIP and Linux release archive are available from GitHub Releases. The Linux archive contains the `gamepad-desktop` executable; see the [desktop guide](desktop/README.md) for platform setup and known limitations.

## Development commands

The root Makefile provides commands for the desktop app and the browser extension:

```bash
make desktop-check
make desktop-lint
make desktop-format
make desktop-test

make web-install
make web-dev
make web-build
make web-test
```

The browser extension has its own npm project and generated files under `extension/`.

## Releases

Conventional Commits on `main` drive semantic-release. A release uses a `desktop-v*` tag and attaches Windows x64 and Linux x64 desktop packages. The extension is validated and built separately; it is not included in desktop releases.
