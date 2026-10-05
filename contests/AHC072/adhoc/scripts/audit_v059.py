"""v059: 固定保存解の診断、事前条件を満たした場合だけ通常100件を1回評価。"""
from __future__ import annotations

from argparse import Namespace
import csv
import difflib
import json
from pathlib import Path
import re
import shutil

from experiment_batch_support import (ROOT, AUDIT_CASES, LONG_CASES, FLAGS, PARENT_ID,
                                      digest, load_module, write_json, write_csv)

CHILD = 'v059_repair_priority'
PARENT = 'v057_search_reductions'


def normalize_preprocessed(text):
    text = re.sub(r'"[^"\n]*(?:v057_search_reductions|v059_repair_priority|parent_reconstructed)\.cpp"', '"solver.cpp"', text)
    return re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', text)


def static_audit(ctx):
    audit = ctx.audit('v059')
    parent = ctx.source(f'src/bin/{PARENT}.cpp')
    child = ctx.source(f'src/bin/{CHILD}.cpp')
    if digest(parent) != '4bb22c34eca9d76fec12861ebd14d807d6ead618b29e79d42c8f91ce7854e29f':
        raise RuntimeError('v059の親ソースが事前登録と異なる')
    restored = child.read_text()
    for change in reversed(json.loads(ctx.source('adhoc/v059_audit/registered_changes.json').read_text())):
        if restored.count(change['new']) != 1:
            raise RuntimeError('登録差分を一意に逆変換できない')
        restored = restored.replace(change['new'], change['old'], 1)
    if restored != parent.read_text():
        raise RuntimeError('v059に登録外の差分がある')
    reconstructed = ctx.binary('parent_reconstructed.cpp')
    reconstructed.write_text(restored)
    (audit / 'source.diff').write_text(''.join(difflib.unified_diff(parent.read_text().splitlines(True), child.read_text().splitlines(True))))
    binaries = {}
    for local, mode in ((True, 'local'), (False, 'production')):
        binaries[mode] = ctx.compile(f'src/bin/{CHILD}.cpp', CHILD if local else CHILD + '_production', local=local)
        flags = ['-DLOCAL'] if local else ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']
        expanded = {}
        for name, source in (('parent', parent), ('child', child), ('reconstructed', reconstructed)):
            output = audit / f'{mode}_{name}.ii'
            ctx.command([ctx.compiler, *FLAGS, *flags, '-E', '-P', source], output=output)
            expanded[name] = normalize_preprocessed(output.read_text())
        if expanded['parent'] != expanded['reconstructed']:
            raise RuntimeError(f'{mode}: 差分を戻した完全前処理結果が親と一致しない')
        (audit / f'{mode}_preprocessed.diff').write_text(''.join(difflib.unified_diff(
            expanded['parent'].splitlines(True), expanded['child'].splitlines(True))))
    write_json(audit / 'static_verification.json', dict(parent_sha256=digest(parent), child_sha256=digest(child),
               restored_parent_equal=True, fully_preprocessed_modes=['local', 'production']))
    return binaries['local']


def rank_metrics(rows, key):
    top = [r for r in rows if int(r[key]) < 32]
    successful = [r for r in top if r['extract_status'] == 'ok']
    return dict(n=len(top), extracted=len(successful), success_rate=len(successful) / len(top),
                mean_net_removed=sum(int(r['net_removed']) for r in successful) / len(successful) if successful else 0,
                inserted=sum(r['insert_status'] == 'ok' for r in top))


def diagnose(ctx, problem_type):
    audit = ctx.audit('v059')
    binary = ctx.compile('adhoc/bin/check_v059_repair_priority.cpp', 'check_v059_repair_priority')
    summaries, sources = [], []
    for case in AUDIT_CASES:
        inp = ctx.parent(f'cases/{case}/input.txt')
        problem = problem_type.read(inp)
        plans = [ctx.parent(f'cases/{case}/seeds/00.txt'), ctx.parent(f'cases/{case}/best.txt'),
                 ctx.saved(f'results/out/{PARENT}/{case}.txt')]
        unique = {}
        for plan in plans:
            canonical = ''.join(' '.join(s.split()) + '\n' for s in plan.read_text().splitlines() if s.strip())
            unique.setdefault(canonical, []).append(str(plan.relative_to(ctx.data)))
        for index, (plan, origins) in enumerate(unique.items()):
            folder = audit / case / f'plan_{index:02d}'
            folder.mkdir(parents=True)
            saved = folder / 'input_plan.txt'
            saved.write_text(plan)
            original = problem.replay(plan)
            sources.append(dict(case=case, index=index, origins=origins, T=original['T'], sha256=digest(saved)))
            print(f'v059 診断 {case}/{index}: {original["T"]}手', flush=True)
            ctx.command([binary, saved, folder], input_path=inp, output=folder / 'probe.log', timeout=900)
            report = json.loads((folder / 'summary.json').read_text())
            rows = list(csv.DictReader((folder / 'candidates.csv').open()))
            for generated in (folder / 'plans').glob('*.txt'):
                metrics = problem.replay(generated.read_text())
                row = rows[int(generated.stem)]
                if metrics['T'] != int(row['final_T']):
                    raise RuntimeError('診断出力と独立再生の手数が異なる')
            old, new = rank_metrics(rows, 'old_rank'), rank_metrics(rows, 'new_rank')
            gate = report['deadlines'] == 0 and report['rank_changed'] > 0 and (
                new['success_rate'] > old['success_rate'] or
                (new['success_rate'] == old['success_rate'] and new['mean_net_removed'] > old['mean_net_removed']))
            summaries.append(dict(case=case, plan_index=index, diagnostic=report, old=old, new=new, gate=gate))
    write_json(audit / 'saved_plans.json', sources)
    eligible = sorted({r['case'] for r in summaries if r['case'] in LONG_CASES and r['gate']})
    if not any(r['diagnostic']['penalized'] > 0 and r['diagnostic']['rank_changed'] > 0 for r in summaries):
        raise RuntimeError('v059の減点・順位変更が一度も発動していない')
    report = dict(plans=summaries, eligible_long_cases=eligible, evaluation_gate=len(eligible) >= 2)
    write_json(audit / 'diagnostic_verification.json', report)
    return report


def evaluate(ctx, binary, problem_type, *, version='v059', child=CHILD, label_prefix='repair_priority',
             require_long_gain=True, mechanism_key=None):
    # The batch owns results/.eval.lock for its entire lifetime. Call the
    # pipeline's already-locked entry, using frozen binaries and frozen inputs.
    ev = load_module(ctx.source('scripts/eval.py'), version + '_frozen_eval')
    ev.ROOT_DIR = ROOT
    ev.DEFAULT_INPUT_DIR = ctx.saved('tools/in')
    ev.SOLVER_BIN_DIR = binary.parent
    ev.TOOLS_BIN_DIR = ctx.saved('tools/target/release')
    ev.SUMMARY_CSV = ROOT / 'results/score_summary.csv'
    ev.DETAIL_CSV = ROOT / 'results/score_detail.csv'
    ev.RECORDS_JSONL = ROOT / 'results/eval_records.jsonl'
    # Building was completed and audited above; avoid rebuilding mutable files.
    def check_solver(name, *, local_enabled):
        if name != child or not local_enabled or not binary.is_file():
            raise RuntimeError('凍結済みLOCAL実行ファイルがない')
    def check_tool(name):
        if name != 'vis' or not (ev.TOOLS_BIN_DIR / name).is_file():
            raise RuntimeError('凍結済み公式scorerがない')
    ev.build_solver, ev.build_tool = check_solver, check_tool
    class InterruptibleExecutor(ev.ThreadPoolExecutor):
        def __exit__(self, exc_type, exc, traceback):
            # Keep normal evaluation scheduling intact. On interruption, let
            # at most the two running cases finish and cancel pending cases.
            self.shutdown(wait=True, cancel_futures=exc_type is not None)
            return False
    ev.ThreadPoolExecutor = InterruptibleExecutor
    label = label_prefix + '_' + ctx.id
    args = Namespace(bin_name=child, dry_run=False, verbose=True, jobs=2, label=label, no_local=False)
    analysis = ctx.analysis(version)
    try:
        code = ev.run_eval_locked(args, sorted(ev.DEFAULT_INPUT_DIR.glob('*.txt')), 'tools/in', True, True)
    finally:
        outputs = ROOT / 'results/out' / child
        if outputs.is_dir():
            shutil.copytree(outputs, analysis / 'outputs')
    if code:
        raise RuntimeError(f'{version}通常評価に失敗した。eval_records.jsonlを確認すること')
    records = [json.loads(line) for line in ev.RECORDS_JSONL.read_text().splitlines() if line.strip()]
    records = [r for r in records if r['bin'] == child and r['label'] == label]
    baseline = json.loads((ctx.data / 'baseline_records.json').read_text())
    previous = {r['case_name']: r for r in baseline}
    if len(records) != 100 or len({r['run_id'] for r in records}) != 1:
        raise RuntimeError(f'{version}の評価100件を一意に照合できない')
    rows, errors, mechanism = [], [], []
    for r in records:
        case = Path(r['case_name']).stem
        out = ROOT / r['stdout_path']
        m = problem_type.read(ctx.saved('tools/in/' + r['case_name'])).replay(out.read_text())
        if r['status'] != 'ok' or m['T'] != r['score']:
            raise RuntimeError(f'{case}: 独立再生と通常評価が一致しない')
        stderr = out.with_suffix(out.suffix + '.err').read_text()
        counts = {k: int(v) for k, v in re.findall(r'\[summary.count\] (\S+)=(-?\d+)', stderr)}
        times = {k: float(v) for k, v in re.findall(r'\[summary.time_ms\] (\S+)=([\d.]+)', stderr)}
        for key in ('construction_errors', 'lns_errors', 'lns_invalid_candidates', 'baseline_recovery', 'final_recovery'):
            if key not in counts or counts[key] != 0:
                errors.append((case, key, counts.get(key)))
        if counts.get('board_pool_free_at_end') != counts.get('board_pool_slots') or 'diagnostic:' in stderr:
            errors.append((case, 'diagnostic_or_board_pool', None))
        mechanism.append(dict(case=case, counts=counts, times_ms=times))
        rows.append(dict(case=case, parent_T=previous[r['case_name']]['score'], T=r['score'],
                         saved=previous[r['case_name']]['score']-r['score'], elapsed_ms=r['elapsed']))
    analysis = ctx.analysis(version)
    write_csv(analysis / 'comparison.csv', rows)
    write_json(analysis / 'mechanism.json', mechanism)
    total = sum(r['saved'] for r in rows)
    long_saved = sum(r['saved'] for r in rows if r['case'] in LONG_CASES)
    max_elapsed = max(r['elapsed_ms'] for r in rows)
    special = next(r['T'] for r in rows if r['case'] == '0000')
    mechanism_saved = sum(r['counts'].get(mechanism_key, 0) for r in mechanism) if mechanism_key else None
    result = dict(status='evaluated', run_id=records[0]['run_id'], parent_run_id=PARENT_ID,
                  saved=total, long_saved=long_saved, max_elapsed_ms=max_elapsed, case_0000_T=special,
                  errors=errors, mechanism_saved=mechanism_saved,
                  adopted=not errors and total > 0 and (not require_long_gain or long_saved > 0) and
                  (mechanism_key is None or mechanism_saved > 0) and max_elapsed <= 2000 and special <= 43)
    write_json(analysis / 'summary.json', result)
    if errors:
        raise RuntimeError(f'{version}の機構検証でエラー: {errors[:5]}')
    return result


def run(ctx):
    worker = load_module(ctx.source('adhoc/scripts/v060_worker.py'), 'v059_saved_plan_reader')
    binary = ctx.binary(CHILD)
    if not (ctx.audit('v059') / 'static_verification.json').is_file():
        raise RuntimeError('一括入口でビルド・差分照合を先に完了すること')
    diagnostic = diagnose(ctx, worker.Problem)
    if not diagnostic['evaluation_gate']:
        result = dict(status='evaluation_skipped', adopted=False, reason='事前登録した診断条件を満たさない',
                      eligible_long_cases=diagnostic['eligible_long_cases'])
        write_json(ctx.analysis('v059') / 'summary.json', result)
        print('v059: 診断条件未達のため通常100件を省略する。設定の変更・再試行は行わない。', flush=True)
        return result
    return evaluate(ctx, binary, worker.Problem)
