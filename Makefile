.DEFAULT_GOAL := help

NPM ?= npm
CARGO ?= cargo
ZIP ?= zip
DESKTOP_MANIFEST := desktop/Cargo.toml
LINUX_TARGET ?= x86_64-unknown-linux-gnu
WINDOWS_TARGET ?= x86_64-pc-windows-gnu
MINGW_BIN ?= $(firstword $(wildcard $(HOME)/.local/share/gamepad-navigator/toolchains/mingw/usr/bin))
WINDOWS_DIST_DIR := desktop/dist
WINDOWS_BINARY := desktop/target/$(WINDOWS_TARGET)/release/gamepad-desktop.exe
WINDOWS_PACKAGE := $(WINDOWS_DIST_DIR)/GamepadNavigator-Windows-x64.zip

.PHONY: help dev run build check lint format format-check test \
	web-install web-dev web-build web-package web-typecheck web-lint web-format web-format-check web-test \
	desktop-run desktop-build desktop-release desktop-linux desktop-windows \
	desktop-check desktop-lint desktop-format desktop-format-check desktop-test desktop-preview

help:
	@printf '%s\n' \
		'Gamepad Navigator / BedArcade' \
		'' \
		'Main: make dev | run | build | check | lint | format | format-check | test' \
		'  dev            Run the browser extension in WXT development mode' \
		'  run            Run the Rust desktop app' \
		'  build          Build the extension and the desktop app for this host' \
		'  check          Type-check the extension and check the Rust app' \
		'  lint           Lint the extension and run Clippy' \
		'  format         Format TypeScript and Rust files' \
		'  format-check   Check formatting without changing files' \
		'  test           Run browser E2E and Rust tests' \
		'' \
		'Extension: web-install, web-dev, web-build, web-package, web-typecheck, web-lint, web-format, web-format-check, web-test' \
		'Desktop:   desktop-run, desktop-build, desktop-release, desktop-linux, desktop-windows, desktop-check, desktop-lint, desktop-format, desktop-format-check, desktop-test, desktop-preview' \
		'' \
		'desktop-windows builds and packages x64 Windows; install the Rust target, MinGW, CMake, and zip first.'

dev: web-dev

run: desktop-run

build: web-build desktop-release

check: web-typecheck desktop-check

lint: web-lint desktop-lint

format: web-format desktop-format

format-check: web-format-check desktop-format-check

test: web-test desktop-test

web-install:
	$(NPM) install

web-dev:
	$(NPM) run dev

web-build:
	$(NPM) run build

web-package: web-build
	$(NPM) run zip

web-typecheck:
	$(NPM) run typecheck

web-lint:
	$(NPM) run lint

web-format:
	$(NPM) run format

web-format-check:
	$(NPM) run format:check

web-test:
	$(NPM) run test:e2e

desktop-run:
	$(CARGO) run --manifest-path $(DESKTOP_MANIFEST)

desktop-build:
	$(CARGO) build --manifest-path $(DESKTOP_MANIFEST)

desktop-release:
	$(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release

desktop-linux:
	$(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release --target $(LINUX_TARGET)

desktop-windows:
	PATH="$(if $(MINGW_BIN),$(MINGW_BIN):)$$PATH" $(CARGO) build --manifest-path $(DESKTOP_MANIFEST) --release --target $(WINDOWS_TARGET)
	mkdir -p $(WINDOWS_DIST_DIR)
	cp $(WINDOWS_BINARY) $(WINDOWS_DIST_DIR)/GamepadNavigator.exe
	$(ZIP) -q -j -FS $(WINDOWS_PACKAGE) $(WINDOWS_DIST_DIR)/GamepadNavigator.exe

desktop-check:
	$(CARGO) check --manifest-path $(DESKTOP_MANIFEST)

desktop-lint:
	$(CARGO) clippy --manifest-path $(DESKTOP_MANIFEST) --all-targets -- -D warnings

desktop-format:
	$(CARGO) fmt --manifest-path $(DESKTOP_MANIFEST)

desktop-format-check:
	$(CARGO) fmt --manifest-path $(DESKTOP_MANIFEST) -- --check

desktop-test:
	$(CARGO) test --manifest-path $(DESKTOP_MANIFEST)

desktop-preview:
	$(CARGO) run --manifest-path $(DESKTOP_MANIFEST) --example keyboard_preview
