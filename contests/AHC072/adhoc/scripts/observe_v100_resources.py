#!/usr/bin/env python3
"""実行中の学習に手を加えず、macOSの資源利用と工程名を短時間記録する。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import plistlib
import statistics
import subprocess
import time


def gpu_usage():
    result = subprocess.run(
        ['/usr/sbin/ioreg', '-a', '-r', '-c', 'AGXAccelerator', '-d', '1'],
        capture_output=True, check=True,
    )
    devices = plistlib.loads(result.stdout)
    return [d.get('PerformanceStatistics', {}) for d in devices]


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--pid', type=int, required=True)
    p.add_argument('--status', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--seconds', type=float, default=180.)
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rows = []
    start = time.monotonic()
    with (args.output / 'samples.jsonl').open('w') as stream:
        while time.monotonic() - start < args.seconds:
            process = subprocess.run(
                ['ps', '-p', str(args.pid), '-o', 'pcpu=', '-o', 'rss='],
                capture_output=True, text=True,
            )
            if process.returncode:
                break
            cpu, rss = process.stdout.split()
            current = json.loads(args.status.read_text())
            row = dict(time=datetime.now().astimezone().isoformat(),
                       seconds=time.monotonic()-start,
                       phase=current['stage'], status_updated_at=current['updated_at'],
                       cpu_percent=float(cpu), rss_kib=int(rss), gpu=gpu_usage())
            rows.append(row)
            stream.write(json.dumps(row)+'\n')
            stream.flush()
            time.sleep(2.)
    summary = {}
    for phase in sorted({r['phase'] for r in rows}):
        part = [r for r in rows if r['phase'] == phase]
        utilization = [d['Device Utilization %'] for r in part for d in r['gpu']
                       if 'Device Utilization %' in d]
        summary[phase] = dict(
            samples=len(part), cpu_percent_mean=statistics.mean(r['cpu_percent'] for r in part),
            gpu_device_percent_mean=statistics.mean(utilization) if utilization else None,
            gpu_device_percent_min=min(utilization) if utilization else None,
            gpu_device_percent_max=max(utilization) if utilization else None,
        )
    result = dict(pid=args.pid, elapsed_seconds=time.monotonic()-start, phases=summary,
                  limitations=['GPU値は機械全体。学習プロセス専用の値ではない。',
                               '工程名は最大約30秒古く、切替付近の標本は混在する。',
                               'psのCPU使用率はOSの平均値であり瞬間値ではない。'])
    (args.output / 'summary.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
