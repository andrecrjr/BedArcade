# Wayland overlay performance comparison

Measured on 2026-10-22. This is an isolated overlay benchmark, not a measurement of the complete desktop app.

## Method

- Baseline source: `285813b`; optimized source: `c5e6740`.
- Release builds with rustc 1.97.1 (8bab26f4f 2026-07-14).
- CPU: Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz; kernel: Linux 7.2.6-76070206-generic x86_64 GNU/Linux.
- Labwc 0.7.1 / wlroots 0.17.1, one headless output, Pixman software rendering.
- The normal Wayland socket refused connections and `/dev/uinput` was not writable. Compositor binaries were extracted under `/tmp`; no system packages were installed.
- Each process warms up for 2 seconds, then measures 60 seconds of actual `WaylandKeyboardOverlay::update` calls, sleeping 8 ms after each call as the runtime does.
- Three trials per version/scenario; version order alternates by trial. The keyboard is stationary and no input is injected.
- Update statistics exclude warmup. CPU uses child-process user + system time from Python `resource.getrusage`, divided by the entire process elapsed time, including startup/warmup. 100% means one fully occupied CPU core. The compositor is excluded.

## Results

Values below are medians across the three trials. Update time is the median of the per-trial means; p95 is the median of the per-trial p95 values.

| Scenario | Mean update before → after (µs) | Reduction | p95 before → after (µs) | Process CPU before → after |
| --- | ---: | ---: | ---: | ---: |
| closed | 25.548 → 17.393 | 31.9% | 36.783 → 23.579 | 0.817% → 0.719% |
| open | 25.218 → 17.472 | 30.7% | 35.070 → 23.752 | 0.854% → 0.776% |

All three optimized runs had lower mean update times than all three baseline runs in each scenario. Absolute CPU savings are approximately 0.1 percentage point of one core in this harness. This supports a small reduction in recurring overlay work; it does not establish a comparable percentage improvement for the complete app, battery life, or controller latency.

Raw records: [overlay-performance.jsonl](overlay-performance.jsonl).

## Regression coverage

The compositor regression test failed before the optimization (`idle update`: 2 position calculations instead of 1) and passed afterward. It checks idle updates, visual-only changes, focused input appearance/movement/disappearance, keyboard resizing/docking, and invalidation through each output callback. The monitor callbacks are invoked by the test; physical monitor hotplug was not exercised.

The normal suite passed: 55 tests, 6 ignored. The compositor regression test is one of those ignored tests and was also run explicitly, passing. Cargo check, Clippy with warnings denied, rustfmt check (toolchain 1.97.0), and the release benchmark build passed.

## Reproduce on a supported Wayland desktop

A compositor supporting `wlr-layer-shell` is required. The benchmark directly instantiates the real overlay without creating the input-injection platform.

Build the baseline and current revision in separate checkouts:

```sh
rtk git worktree add --detach /tmp/gamepad-overlay-before 285813b
rtk cargo build --manifest-path /tmp/gamepad-overlay-before/desktop/Cargo.toml --release --example overlay_benchmark
rtk cargo build --manifest-path desktop/Cargo.toml --release --example overlay_benchmark
```

Run each version/scenario three times, alternating version order between trials. Keep the same compositor, monitors, build profile and power settings. Allow other compilation tasks to finish before measuring.

```sh
rtk proxy /usr/bin/time -f 'user=%U system=%S elapsed=%e' /tmp/gamepad-overlay-before/desktop/target/release/examples/overlay_benchmark closed 60
rtk proxy /usr/bin/time -f 'user=%U system=%S elapsed=%e' desktop/target/release/examples/overlay_benchmark closed 60
```

Repeat with `open` instead of `closed`. The executable prints JSON containing the mean, p50 and p95 update times. Calculate process CPU as `100 * (user + system) / elapsed`; GNU time prints CPU seconds at lower precision than the raw records above.

Run the compositor regression separately:

```sh
rtk cargo test --manifest-path desktop/Cargo.toml --lib positioning_tracks_geometry -- --ignored --nocapture --test-threads=1
```

A full-app measurement with a physical controller and real monitor changes remains necessary to assess total CPU usage and user-perceived latency.
