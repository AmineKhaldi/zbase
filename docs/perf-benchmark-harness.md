# GPUI Performance Benchmark Harness

This project now includes an in-app benchmark harness for repeatable UI performance capture.

## Manual capture (drive it yourself)

1. Run the app normally.
2. Press `cmd-shift-b` to start a capture session.
3. Drive the exact scenario you care about (for example: scroll a busy chat for 20s).
4. Press `cmd-shift-b` again to stop capture.

When the session stops, the app logs:

- run metadata (label, duration, refresh count, backend poll/event counts)
- timing stats (`n`, `mean`, `p50`, `p95`, `p99`, `max`) for:
  - `render_total`
  - `sidebar_render`
  - `main_panel_render`
  - `right_pane_render`
  - `dispatch_ui_action`
  - `sync_models_from_store`
  - `drain_backend_events`

## Env-driven capture

Set these env vars before launch:

- `ZBASE_BENCH_AUTOSTART=1` to start capture automatically.
- `ZBASE_BENCH_DURATION_SECS=<seconds>` to auto-stop after a fixed duration.
- `ZBASE_BENCH_LABEL=<label>` to tag a run (for example `baseline`, `after-virtualization`).
- `ZBASE_BENCH_SCENARIO=<name>` to tag the workflow (for example `timeline-scroll`).
- `ZBASE_BENCH_OUTPUT=<path>` to append CSV results.
- `ZBASE_BENCH_EXIT_ON_STOP=1` to exit the app process automatically when capture stops.
- `ZBASE_INPUT_PROFILE=1` to emit detailed text-input timing logs.
- `ZBASE_INPUT_PROFILE_MIN_MS=<ms>` to control text-input log threshold (`0` logs all samples).

Example:

```bash
ZBASE_BENCH_AUTOSTART=1 \
ZBASE_BENCH_DURATION_SECS=25 \
ZBASE_BENCH_LABEL=baseline \
ZBASE_BENCH_SCENARIO=timeline-scroll \
ZBASE_BENCH_OUTPUT=/tmp/zbase-perf.csv \
ZBASE_BENCH_EXIT_ON_STOP=1 \
cargo run --release
```

The CSV file is append-only and contains one row per capture session.

## Scripted workload mode (automatable by agent/CI)

To make runs repeatable without manual interaction, enable scripted workload mode.

- `ZBASE_BENCH_USE_DEMO=1` starts from demo UI data (stable dataset).
- `ZBASE_BENCH_SKIP_BACKEND=1` disables backend boot/poll noise.
- `ZBASE_BENCH_SCRIPT=timeline_scroll` drives a chat-scroll-like render workload.
- `ZBASE_BENCH_SCRIPT=profile_scroll` drives profile-pane scrolling workload (pair with `ZBASE_BENCH_PROFILE_USER`).
- `ZBASE_BENCH_SCRIPT=sidebar_filter` drives sidebar filtering churn.
- `ZBASE_BENCH_SCRIPT=sync_heavy` forces `sync_models_from_store` each tick.
- `ZBASE_BENCH_SCRIPT=composer_typing` drives continuous composer input updates.
- `ZBASE_BENCH_SCRIPT=composer_paste` repeatedly pastes multi-line text into the composer.
- `ZBASE_BENCH_SCRIPT=thread_open` repeatedly opens thread roots to benchmark thread-panel latency.
- `ZBASE_BENCH_SCRIPT_TICK_MS=<ms>` controls script tick cadence (default `16`).
- `ZBASE_BENCH_TIMELINE_MESSAGES=<count>` inflates synthetic timeline history to stress large-chat scrolling.
- `ZBASE_BENCH_PROFILE_USER=<username>` opens that profile in the right pane during scripted runs (useful for profile performance work).
- `ZBASE_BENCH_THREAD_CONVERSATION_ID=<conversation-id>` forces scripted thread-open runs to navigate to that conversation first.
- `ZBASE_BENCH_THREAD_ROOT_ID=<message-id>` pins scripted thread-open runs to a single root.
- `ZBASE_BENCH_THREAD_ROOT_IDS=<id1,id2,...>` cycles scripted thread-open runs across a comma-separated set of roots.
- `ZBASE_THREAD_OPEN_PROFILE=1` emits per-open breakdown logs (`dispatch_ui_open_thread`, `backend_load_thread`, and scripted tick totals).
- `ZBASE_PROFILE_DISABLE_SOCIAL_VIRTUALIZATION=1` disables profile social-list virtualization for A/B perf comparisons.
- `ZBASE_INPUT_DISABLE_LAYOUT_CACHE=1` disables text layout reuse in `TextField` for A/B comparisons.
- `ZBASE_INPUT_USE_LEGACY_REPLACE=1` uses the old string-concatenation replace path in `TextField`.
- `ZBASE_INPUT_CLEAR_BOUNDS_ON_EDIT=1` restores the old behavior that clears text-field bounds cache on each edit.
- `ZBASE_INPUT_DISABLE_UNDO_COALESCE=1` disables adjacent single-character undo coalescing.

Automated example:

```bash
ZBASE_BENCH_USE_DEMO=1 \
ZBASE_BENCH_SKIP_BACKEND=1 \
ZBASE_DATA_DIR=/tmp/zbase-demo-benchmark-data \
ZBASE_BENCH_SCRIPT=timeline_scroll \
ZBASE_BENCH_TIMELINE_MESSAGES=20000 \
ZBASE_BENCH_SCRIPT_TICK_MS=16 \
ZBASE_BENCH_AUTOSTART=1 \
ZBASE_BENCH_DURATION_SECS=25 \
ZBASE_BENCH_LABEL=after-change \
ZBASE_BENCH_SCENARIO=timeline_scroll \
ZBASE_BENCH_OUTPUT=/tmp/zbase-perf.csv \
ZBASE_BENCH_EXIT_ON_STOP=1 \
cargo run --release --features demo-fixtures
```

To isolate sync costs without scroll noise, switch to:

```bash
ZBASE_BENCH_SCRIPT=sync_heavy
```

## Validated UI review (2026-11-03)

The reproducible runner is `scripts/benchmark_ui.py`. Review reports and raw
measurements are local artifacts; live captures are not checked into the repository.

Additional scenarios are `navigation`, `quick_switcher`, `global_search`, and
`idle`. They invoke the app's normal controller/input paths. Thread-open and
paste samples now record actual operations rather than mostly idle ticks.
Demo mode seeds the central store as well as the view models, including inflated
history, and dismisses its startup splash. Before this correction, a sync workload
could erase its demo fixture and report timings for an empty view.

Additional controls:

- `ZBASE_BENCH_WARMUP_SECS`: delay capture to separate startup from steady work.
- `ZBASE_BENCH_READ_ONLY=1`: in a benchmark session, block chat/account mutations
  and mark-read commands, attachment hydration, and the full-history network crawl.
  Ordinary avatar, emoji, link-preview and remote image reads remain enabled.
- `ZBASE_BENCH_FORCE_DRAW=1`: force a full GPUI CPU scene/layout build on each
  scripted tick without GPU presentation. This works with a locked/occluded
  display. It bypasses view reuse and is a full-window stress measurement, not
  typical incremental frame time or a displayed frame-rate measurement.

New metrics are `scenario_step`, `ui_loop_delay`, `backend_delivery_delay`, and
`frame_build`. The delivery probe is a synthetic marker through the same backend
queue and foreground drain; it measures handoff delay, not server round-trip or
end-to-end message latency. `bench.state` logs message/conversation counts and
route/conversation agreement without logging message bodies. The runner rejects
crashes, timeouts, incomplete captures, route/conversation mismatches, and active
workloads without rendering samples. Idle captures may legitimately have none.

Build with the optional runtime-shader feature if Xcode's offline Metal toolchain
is unavailable:

```sh
cargo build --release --bin zbase --features gpui/runtime_shaders
```

A synthetic run needs no live account:

```sh
python3 scripts/benchmark_ui.py \
  --binary /path/to/target/release/zbase \
  --output /tmp/zbase-demo-benchmark \
  --suite demo --counts 1000,5000 --duration 10 --warmup 3 --offscreen
```

For live read-only runs, close zBase before making a disposable copy of its local
cache. On macOS, APFS cloning avoids copying the database's bytes initially:

```sh
BENCH_DATA="$(mktemp -d /tmp/zbase-benchmark-data.XXXXXX)"
mkdir -p "$BENCH_DATA/keybase"
cp -cR "$HOME/Library/Application Support/io.zbase.zbase/keybase/rocksdb" \
  "$BENCH_DATA/keybase/rocksdb"
python3 scripts/benchmark_ui.py \
  --binary /path/to/target/release/zbase \
  --output /tmp/zbase-live-benchmark --suite live \
  --live-data "$BENCH_DATA" --duration 10 --offscreen
```

Use `--scenarios navigation,thread_open` to select workloads and `--repeats` for
additional captures. Keep profiler runs separate from baseline captures. Omit
`--offscreen` on an unlocked desktop when measuring normal rendering callbacks;
use manual captures and native inspection for visual/input correctness and GPU
frame pacing. A benchmark process exits abruptly after saving its capture, so
always use disposable data for live runs.

The 2026-11-03 run completed 17 optimized offscreen captures: 10 synthetic
captures at 1,000/5,000 messages and 7 live read-only captures, with an inbox that
expanded to 1,075 conversations. All 81 sampled route/conversation checks agreed;
there were no crashes or capture timeouts. Two remote Giphy image loads failed.
Earlier zero-render runs after the Mac locked were excluded. Visible interaction,
GPU presentation, filter semantics, reconnect fault injection and long memory
soaks were not validated by these captures. All 194 tests passed after adding the
instrumentation.

## Fixture isolation (2026-11-04)

Normal builds exclude `src/models/demo.rs` and synthetic timeline inflation.
Production models are constructed empty; navigation never seeds a demo timeline,
search result, activity item, participant, attachment, or typing indicator.
To run synthetic/demo benchmarks, explicitly build a separate binary with:

```sh
cargo build --release --bin zbase --features gpui/runtime_shaders,demo-fixtures
```

Demo mode additionally requires `ZBASE_BENCH_USE_DEMO=1`,
`ZBASE_BENCH_SKIP_BACKEND=1`, and an isolated `ZBASE_DATA_DIR`. The Python runner
supplies these environment variables for demo workloads. Live benchmarks use a
normal build with a disposable cache. Never point a demo build at production data.

Benchmark state samples now validate that every displayed message belongs to the
active conversation and exists in the store, and that typing indicators match
the store. Navigation records these checks before asynchronous responses arrive.
