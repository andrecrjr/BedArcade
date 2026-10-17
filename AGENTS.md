# Repository Guidelines

## Project Structure and Module Organization

- `entrypoints/content.ts` runs the gamepad loop in supported web pages, manages the focus outline, and activates selected controls.
- `entrypoints/background.ts` handles browser tab actions such as back, next tab, and previous tab.
- `lib/spatial.ts` finds visible, enabled candidates and chooses the nearest target by direction.
- `lib/gamepad.ts` reads real browser gamepad state in production and provides controlled input frames to E2E builds.
- `tests/e2e/` contains Playwright tests against local pages loaded with the WXT extension.
- `wxt.config.ts` defines the extension manifest; `tsconfig.json` enables strict TypeScript checking.
- `README.md` documents setup and manual use. WXT generates the unpacked extension under `.output/chrome-mv3`.

## Build, Test, and Development Commands

- `npm install` installs the project dependencies.
- `npm run dev` starts WXT development mode.
- `npm run typecheck` runs TypeScript without emitting files.
- `npm run lint` runs Oxlint; `npm run format` formats project files with Oxfmt, and `npm run format:check` checks formatting without edits.
- `npm run build` creates a production extension build.
- `npm run test:e2e` builds the WXT `e2e` mode and runs Playwright against Chromium; run `npx playwright install chromium` once to install its browser.
- `npm run zip` packages the build for distribution.

## Coding Style and Naming Conventions

Use strict TypeScript and preserve the existing two-space indentation, single quotes, and semicolons. Prefer explicit types at browser API and message boundaries. Use `camelCase` for variables and functions; keep module filenames lowercase. No formatter or linter is configured, so keep changes consistent with nearby code and run `npm run typecheck`.

## Testing Guidelines

Before submitting, run `npm run format:check`, `npm run lint`, `npm run typecheck`, `npm run test:e2e`, and `npm run build`. E2E tests use deterministic standard-mapped input frames and verify focus movement, activation, pause/resume, browser history, and tab switching. Also use the manual gamepad walkthrough in `README.md` when validating physical controllers; automated coverage currently targets Chromium and standard mappings. No coverage threshold is configured.

## Commits and Pull Requests

This checkout has no Git metadata, so no project-specific commit convention can be verified. Use a short imperative commit subject. A pull request should describe the user-visible change, list validation performed, link a related issue when available, and include screenshots for visible UI changes.
