"""v059–v061の固定条件実験で共有する入出力・ビルド。探索方針は持たない。"""
from __future__ import annotations

import csv
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys

ROOT = Path(os.environ.get('AHC072_BATCH_ROOT', Path(__file__).resolve().parents[2])).resolve()
PARENT_RUN = Path('results/long_search/v800/20260929T150407_c945d3d4')
PARENT_ID = '20260929T145653+0900_v057_search_reductions_af817c'
LONG_CASES = ('0014', '0015', '0030', '0034', '0042', '0047')
AUDIT_CASES = ('0001', '0014', '0015', '0027', '0030', '0034', '0042', '0044', '0047')
RESTART_CASES = ('0001', '0014', '0015', '0027', '0030', '0034', '0042', '0047')
THREAD_ENV = {k: '1' for k in ('OMP_NUM_THREADS', 'OMP_THREAD_LIMIT', 'OPENBLAS_NUM_THREADS',
                              'MKL_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS', 'NUMEXPR_NUM_THREADS')}
FLAGS = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native', '-pthread',
         '-ftrivial-auto-var-init=zero', '-fopenmp']


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temporary.replace(path)


def write_csv(path, rows, fields=None):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=fields or list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class Context:
    def __init__(self, run):
        self.run = Path(run).resolve()
        self.id = self.run.name
        self.snapshot = self.run / 'snapshot'
        self.data = self.run / 'data'
        self.manifest = json.loads((self.run / 'manifest.json').read_text())
        self.env = os.environ.copy() | THREAD_ENV
        if sys.platform == 'darwin':
            self.env.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
            self.env.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
        self.compiler = shutil.which(self.env.get('CXX', 'g++-15'))
        if not self.compiler:
            raise RuntimeError('GCC 15が見つからない。CXXにコンパイラを指定すること。')

    def source(self, relative):
        return self.snapshot / relative

    def saved(self, relative):
        return self.data / relative

    def parent(self, relative=''):
        return self.saved(PARENT_RUN / relative)

    def audit(self, version):
        path = ROOT / 'adhoc' / (version + '_audit') / self.id
        path.mkdir(parents=True, exist_ok=True)
        return path

    def analysis(self, version):
        path = ROOT / 'results/analysis' / version / self.id
        path.mkdir(parents=True, exist_ok=True)
        return path

    def binary(self, name):
        path = self.run / 'build' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        return path

    def command(self, args, *, output, input_path=None, timeout=None, text_input=None):
        """失敗や中断で子プロセスを残さない。実験は呼出側で最大2並列に制限する。"""
        output = Path(output)
        output.parent.mkdir(parents=True, exist_ok=True)
        inp = Path(input_path).open('rb') if input_path is not None else None
        try:
            with output.open('wb') as stream:
                process = subprocess.Popen([str(x) for x in args], cwd=ROOT, env=self.env,
                                           stdin=inp if inp else subprocess.PIPE if text_input is not None else subprocess.DEVNULL,
                                           stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    process.communicate(text_input.encode() if text_input is not None else None, timeout=timeout)
                    if process.returncode:
                        raise RuntimeError(f'終了コード{process.returncode}: {args[0]}。ログ: {output}')
                except BaseException:
                    if process.poll() is None:
                        os.killpg(process.pid, signal.SIGTERM)
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            os.killpg(process.pid, signal.SIGKILL)
                            process.wait()
                    raise
        finally:
            if inp:
                inp.close()

    def compile(self, source, name, *, local=True, extra=()):
        binary = self.binary(name)
        flags = ['-DLOCAL'] if local else ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']
        command = [self.compiler, *FLAGS, *flags, *extra, str(self.source(source)), '-o', str(binary)]
        metadata = self.run / 'build' / (name + '.json')
        if metadata.exists():
            previous = json.loads(metadata.read_text())
            if (previous['command'] != command or previous['source_sha256'] != digest(self.source(source)) or
                    previous['sha256'] != digest(binary)):
                raise RuntimeError(f'既にビルドした実行ファイルの条件が変化した: {name}')
            return binary
        self.command(command, output=self.run / 'build' / (name + '.log'))
        write_json(self.run / 'build' / (name + '.json'),
                   dict(command=command, sha256=digest(binary), source_sha256=digest(self.source(source)),
                        environment={k: self.env[k] for k in (*THREAD_ENV, 'SDKROOT', 'MACOSX_DEPLOYMENT_TARGET') if k in self.env}))
        return binary
