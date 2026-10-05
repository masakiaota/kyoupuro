#!/usr/bin/env python3
"""既存のNN学習記録だけを読む、標準ライブラリ製の監視画面。"""
import argparse
from collections import deque
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import math
from pathlib import Path
import re
import threading
import time
from urllib.parse import parse_qs, urlsplit

ROOT = Path(__file__).resolve().parents[2]
PAGE = Path(__file__).with_suffix('.html')
FILE_CACHE = {}
CACHE_LOCK = threading.Lock()


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        # 学習側が書き換えている瞬間の読み取りは、次の更新で再取得する。
        return {}


def records(path):
    try:
        stat = path.stat()
    except OSError:
        return []
    signature = (stat.st_mtime_ns, stat.st_size)
    with CACHE_LOCK:
        old = FILE_CACHE.get(path)
        if old and old[0] == signature:
            return old[1]
        rows = []
        try:
            for line in path.read_text().splitlines():
                try:
                    row = json.loads(line)
                    if isinstance(row, dict):
                        rows.append(row)
                except ValueError:
                    continue  # 追記途中の末尾を完成した記録として扱わない。
        except OSError:
            return old[1] if old else []
        # 再開時に先行した記録が残る場合は同じ区間の最後の記録を使う。
        by_iteration = {r['iteration']: r for r in rows if 'iteration' in r}
        rows = [by_iteration[k] for k in sorted(by_iteration)]
        FILE_CACHE[path] = (signature, rows)
        return rows


def aggregate(rows):
    episodes = sum(r.get('episodes', 0) for r in rows)
    successes = sum(r.get('successes', 0) for r in rows)
    measured = [r for r in rows if r.get('mean_T_completed') is not None]
    weight = sum(r.get('successes', 0) for r in measured)
    return dict(episodes=episodes, successes=successes,
                mean_T=sum(r['mean_T_completed'] * r.get('successes', 0) for r in measured) / weight if weight else None,
                success_rate=successes / episodes * 100 if episodes else None)


def compact(points, limit=500):
    if len(points) <= limit:
        return points
    return [points[round(i * (len(points) - 1) / (limit - 1))] for i in range(limit)]


def status_info(directory, status, guard_folder=None):
    stamp = status.get('updated_at')
    age = None
    try:
        age = max(0, time.time() - datetime.fromisoformat(stamp).timestamp())
    except (TypeError, ValueError):
        pass
    stage = status.get('stage', '')
    guard_folder = guard_folder or directory / 'guard_train'
    exit_record = read_json(guard_folder / 'exit.json')
    if stage == 'training_completed':
        label = '学習完了'
    elif exit_record:
        label = '停止・終了'
    elif age is not None and age > 180:
        label = '記録更新なし'
    else:
        label = '学習中' if stamp else '記録待ち'
    return dict(label=label, updated_at=stamp, age_seconds=age, stage=stage,
                remaining_seconds=status.get('remaining_seconds') if stage != 'training_completed' else None)


def rl_run(directory, version, window, monitor_root=None):
    folder = directory / 'training'
    rows = records(folder / 'metrics.jsonl')
    status = read_json(folder / 'status.json')
    if not rows and not status:
        return None
    recent = rows[-window:]
    points, pending = [], deque()
    previous = None
    for row in rows:
        pending.append(row)
        if len(pending) > window:
            pending.popleft()
        avg = aggregate(pending)
        speed = None
        if previous:
            delta = row.get('seconds', 0) - previous.get('seconds', 0)
            if delta > 0:
                speed = (row.get('steps', 0) - previous.get('steps', 0)) / delta
        points.append(dict(x=row['iteration'], raw=row.get('mean_T_completed'),
                           mean=avg['mean_T'], success=avg['success_rate'], speed=speed))
        previous = row
    latest = rows[-1] if rows else status
    monitor_root = monitor_root or directory
    launch = read_json(monitor_root / 'launch.json')
    guard_folder = Path(launch['guard_directory']) if launch.get('guard_directory') else monitor_root / 'guard_train'
    guard = read_json(guard_folder / 'status.json')
    config = read_json(directory / 'run_config.json')
    aliases = {'mc_ppo': '完走結果のPPO', 'group': '同じ盤面の4列を比較',
               'control': '逆再生教師なし', 'mixed': '逆再生教師あり'}
    run_id = directory.relative_to(ROOT / 'results/nn_rank').as_posix()
    label = f'{version} · {directory.parent.name} · {aliases.get(directory.name, directory.name)}' if len(run_id.split('/')) > 2 else f'{version} · {directory.name}'
    return dict(id=run_id, label=label, kind='rl', method=config.get('method'),
                source=folder.relative_to(ROOT).as_posix(), memory_shared=monitor_root != directory,
                status=status_info(directory, status, guard_folder), recent=aggregate(recent), total=aggregate(rows),
                intervals=len(recent), latest=latest, points=compact(points), rows=rows[-10:][::-1],
                memory_gb=guard.get('bytes', 0) / 1e9 if guard else None,
                seconds=latest.get('seconds'), steps=latest.get('steps'), iteration=latest.get('iteration'))


def bc_runs(directory, version):
    models = directory / 'models'
    if not models.is_dir():
        return []
    out = []
    for model in sorted(models.iterdir()):
        if not model.is_dir():
            continue
        rows = [read_json(p) for p in sorted(model.glob('metrics_epoch*.json'))]
        rows = [r for r in rows if 'epoch' in r and 'validation' in r]
        if not rows:
            continue
        status = read_json(model / 'status.json')
        points = [dict(x=r['epoch'], train=r['train_fixed']['policy_ce'], val=r['validation']['policy_ce'],
                       train_accuracy=r['train_fixed']['teacher_top1'] * 100,
                       val_accuracy=r['validation']['teacher_top1'] * 100) for r in rows]
        out.append(dict(id=f'{version}/{directory.name}/{model.name}',
                        label=f'{version} · BC · {model.name}', kind='bc',
                        status=status_info(directory, status), latest=rows[-1], points=points,
                        rows=rows[::-1], steps=status.get('steps'), iteration=status.get('epoch'),
                        memory_gb=None, seconds=None))
    return out


def snapshot(window):
    runs = []
    base = ROOT / 'results/nn_rank'
    if base.is_dir():
        directories = []
        for version in sorted(base.iterdir()):
            if not version.is_dir() or not re.fullmatch(r'v\d{3}', version.name):
                continue
            for directory in sorted(version.iterdir()):
                if not directory.is_dir():
                    continue
                directories.append((directory, version.name))
        # 一つの監視工程が別バージョンも実行する場合は、その監視記録を共有する。
        monitors = {}
        for directory, _ in directories:
            launch = read_json(directory / 'launch.json')
            if launch.get('reverse_root'):
                monitors[Path(launch['reverse_root'])] = directory
        for directory, version in directories:
            monitor_root = monitors.get(directory, directory)
            rl = rl_run(directory, version, window, monitor_root)
            if rl:
                runs.append(rl)
            runs.extend(bc_runs(directory, version))
            # 条件別のtraining/まで調べる。教師cases/や巨大な配列は読まない。
            for variant in sorted(directory.iterdir()):
                if variant.is_dir() and (variant / 'training').is_dir():
                    rl = rl_run(variant, version, window, monitor_root)
                    if rl:
                        runs.append(rl)
    runs.sort(key=lambda r: r['status'].get('updated_at') or '', reverse=True)
    return dict(runs=runs, window=window, fetched_at=datetime.now(timezone.utc).isoformat())


def finite_json(value):
    if isinstance(value, float) and not math.isfinite(value):
        return None
    if isinstance(value, dict):
        return {k: finite_json(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [finite_json(v) for v in value]
    return value


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        url = urlsplit(self.path)
        if url.path in ('/', '/index.html'):
            body, mime = PAGE.read_bytes(), 'text/html; charset=utf-8'
        elif url.path == '/api/dashboard':
            try:
                window = int(parse_qs(url.query).get('window', ['30'])[0])
            except ValueError:
                window = 30
            window = window if window in (10, 30, 50, 100) else 30
            body = json.dumps(finite_json(snapshot(window)), ensure_ascii=False, allow_nan=False).encode()
            mime = 'application/json; charset=utf-8'
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header('Content-Type', mime)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.end_headers()
        self.wfile.write(body)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=8769)
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', args.port), Handler)
    print(f'http://127.0.0.1:{server.server_port}/', flush=True)
    server.serve_forever()
