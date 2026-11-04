#!/usr/bin/env python3
"""Run sequential GPUI benchmark workloads with process resource sampling.

Live runs require an explicitly supplied, disposable copy of zBase's data directory.
They enable read-only benchmark mode: no mark-read, sends, edits, reactions, joins,
media hydration, or network history crawl. Render timers measure CPU view-building,
not GPU presentation or physical keyboard-to-display latency.
"""
import argparse
import csv
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--suite', choices=['demo', 'live', 'all'], default='demo')
    parser.add_argument('--live-data', type=Path)
    parser.add_argument('--counts', default='1000,20000')
    parser.add_argument('--duration', type=int, default=10)
    parser.add_argument('--warmup', type=int, default=3)
    parser.add_argument('--repeats', type=int, default=1)
    parser.add_argument('--scenarios', help='Optional comma-separated scenario filter')
    parser.add_argument('--offscreen', action='store_true', help='Force CPU scene/layout builds without GPU presentation')
    args = parser.parse_args()
    if args.suite != 'demo' and args.live_data is None:
        parser.error('Live runs require --live-data pointing to a disposable cache copy')
    if args.duration <= 0 or args.warmup < 0 or args.repeats <= 0:
        parser.error('duration/repeats must be positive and warmup must be nonnegative')
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f'App executable not found: {binary}')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    cases = []
    if args.suite != 'live':
        for count in [int(value) for value in args.counts.split(',')]:
            for scenario in ['timeline_scroll', 'composer_typing', 'composer_paste', 'sidebar_filter', 'sync_heavy']:
                cases.append((f'demo-{scenario}-{count}', scenario, count, True, args.warmup))
    if args.suite != 'demo':
        cases.append(('live-startup', 'idle', 0, False, 0))
        for scenario in ['idle', 'navigation', 'quick_switcher', 'global_search', 'thread_open', 'composer_typing']:
            cases.append((f'live-{scenario}', scenario, 0, False, max(8, args.warmup)))
    if args.scenarios:
        selected = set(args.scenarios.split(','))
        cases = [case for case in cases if case[1] in selected]
    if not cases:
        parser.error('No scenarios matched the requested suite/filter')
    summaries = []
    for repeat in range(args.repeats):
        for label, scenario, count, demo, warmup in cases:
            name = f'{label}-r{repeat + 1}'
            run = output / name
            run.mkdir(exist_ok=True)
            environment = {key: value for key, value in os.environ.items() if not key.startswith('ZBASE_BENCH_')}
            environment.update({
                'ZBASE_DATA_DIR': str(run / 'data' if demo else args.live_data.resolve()),
                'ZBASE_BENCH_AUTOSTART': '1', 'ZBASE_BENCH_EXIT_ON_STOP': '1',
                'ZBASE_BENCH_SCRIPT': scenario, 'ZBASE_BENCH_SCRIPT_TICK_MS': '16',
                'ZBASE_BENCH_DURATION_SECS': str(args.duration),
                'ZBASE_BENCH_WARMUP_SECS': str(warmup),
                'ZBASE_BENCH_LABEL': name, 'ZBASE_BENCH_SCENARIO': scenario,
                'ZBASE_BENCH_OUTPUT': str(output / 'timings.csv'),
                'ZBASE_BENCH_FORCE_DRAW': '1' if args.offscreen else '0',
                'ZBASE_BENCH_READ_ONLY': '1', 'ZBASE_THREAD_OPEN_PROFILE': '1' if scenario == 'thread_open' else '0',
            })
            if demo:
                environment.update({'ZBASE_BENCH_USE_DEMO': '1', 'ZBASE_BENCH_SKIP_BACKEND': '1', 'ZBASE_BENCH_TIMELINE_MESSAGES': str(count)})
            started = time.monotonic()
            samples = []
            print(f'START {name}', flush=True)
            with (run / 'app.log').open('w') as log:
                process = subprocess.Popen([str(binary)], env=environment, cwd=run, stdout=log, stderr=subprocess.STDOUT)
                (run / 'process.pid').write_text(str(process.pid))
                timed_out = False
                while process.poll() is None:
                    elapsed = time.monotonic() - started
                    if elapsed > warmup + args.duration + 40:
                        timed_out = True
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait()
                        break
                    stats = subprocess.run(['ps', '-p', str(process.pid), '-o', '%cpu=,rss='], text=True, capture_output=True).stdout.split()
                    if len(stats) == 2:
                        samples.append({'elapsed_s': round(elapsed, 3), 'cpu_percent': float(stats[0]), 'rss_mib': int(stats[1]) / 1024})
                    time.sleep(1)
            (run / 'resources.json').write_text(json.dumps(samples, indent=2))
            log_text = (run / 'app.log').read_text(errors='replace')
            states = [line for line in log_text.splitlines() if 'bench.state' in line]
            bad_states = [line for line in states if any(flag + '=false' in line for flag in ['route_matches', 'conversation_matches', 'message_provenance_matches', 'typing_matches'])]
            steady = [sample for sample in samples if sample['elapsed_s'] >= warmup + 2]
            timing_rows = list(csv.DictReader((output / 'timings.csv').open())) if (output / 'timings.csv').exists() else []
            timing = next((row for row in reversed(timing_rows) if row['label'] == name), {})
            render_samples = int(timing.get('render_total_n', 0))
            summary = {'label': name, 'demo': demo, 'offscreen': args.offscreen, 'messages_requested': count, 'exit_code': process.returncode,
                       'timeout': timed_out, 'wall_seconds': round(time.monotonic() - started, 3),
                       'peak_rss_mib': round(max((sample['rss_mib'] for sample in samples), default=0), 1),
                       'mean_cpu_percent': round(sum(sample['cpu_percent'] for sample in steady) / max(1, len(steady)), 1),
                       'state_samples': len(states), 'state_mismatches': len(bad_states),
                       'panic': 'panicked at' in log_text, 'capture_complete': 'bench.capture.saved' in log_text,
                       'render_samples': render_samples, 'render_valid': scenario == 'idle' or render_samples > 0}
            summaries.append(summary)
            (output / 'runs.json').write_text(json.dumps(summaries, indent=2))
            print('DONE ' + json.dumps(summary), flush=True)
    if summaries:
        with (output / 'runs.csv').open('w', newline='') as file:
            writer = csv.DictWriter(file, fieldnames=summaries[0].keys())
            writer.writeheader()
            writer.writerows(summaries)
    if any(run['exit_code'] != 0 or run['timeout'] or run['panic'] or not run['capture_complete'] or not run['render_valid'] or run['state_mismatches'] > 0 for run in summaries):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
