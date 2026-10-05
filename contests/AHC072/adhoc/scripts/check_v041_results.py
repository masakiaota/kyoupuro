#!/usr/bin/env python3
"""Independently verify the frozen diagnostic and the single v041 evaluation."""
import csv
import hashlib
import json
from pathlib import Path
import re
import sys

from check_v037_results import ERRORS, log_values, verify_output
from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v041_audit'
BIN = 'v041_joint_window'
PARENT = 'v039_exact_board_lns'
PARENT_RUN = '20260928T092503+0900_v039_exact_board_lns_3c2389'


def verify_sources():
    audit = json.loads((AUDIT/'static_verification.json').read_text())
    for name, key in [(BIN, 'solver_sha256'), (PARENT, 'parent_sha256')]:
        assert hashlib.sha256((ROOT/f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for case, digest in audit['input_sha256'].items():
        assert hashlib.sha256((ROOT/'tools/in'/case).read_bytes()).hexdigest() == digest
    return audit


def state_at(case, operations, turn):
    lines = (ROOT/'tools/in'/case).read_text().splitlines()
    N, K = map(int, lines[0].split())
    board = {str((i,j)): ch if ch.islower() else ''
             for i, row in enumerate(lines[1:N+1]) for j, ch in enumerate(row) if ch != '#'}
    for operation in operations[:turn]:
        board.update(operation['after'])
    return board


def diagnostic():
    audit = verify_sources()
    rows = list(csv.DictReader((AUDIT/'diagnostic.csv').open()))
    assert len(rows) == len({r['case'] for r in rows}) == 100
    for row in rows:
        for key in row:
            if key != 'case':
                row[key] = float(row[key]) if key == 'ms' else int(row[key])
        actual = verify_output(row['case'], AUDIT/'compressed_v039'/row['case'])
        assert actual == row['after'] <= row['before']
        assert row['saved'] == row['before'] - row['after']
    # Splice the automatically found replacement into the manually identified
    # window, retaining its 11 unrelated operations and the full original tail.
    original = (ROOT/'adhoc/play_saved_edits_20260928/v039_0060_original.txt').read_text().splitlines()
    replacement = (AUDIT/'manual_window_replacement.txt').read_text().splitlines()
    related = {41,42,43,44,53,55,58}
    modified = original[:41] + replacement + [original[i] for i in range(41,59) if i not in related] + original[59:]
    witness_path = AUDIT/'manual_window_full.txt'
    witness_path.write_text('\n'.join(modified)+'\n')
    assert verify_output('0060.txt',witness_path) <= len(original)-2
    before = replay(ROOT/'tools/in/0060.txt',ROOT/'adhoc/play_saved_edits_20260928/v039_0060_original.txt')
    after = replay(ROOT/'tools/in/0060.txt',witness_path)
    new_boundary = 41+len(replacement)+11
    assert state_at('0060.txt',before['operations'],59) == state_at('0060.txt',after['operations'],new_boundary)
    assert next(r for r in rows if r['case']=='0060.txt')['saved'] >= 2
    mechanism = json.loads((AUDIT/'mechanism.json').read_text())
    result = {'solver_sha256':audit['solver_sha256'],'verified_cases':100,'mechanism':mechanism,
              'saved_moves':sum(r['saved'] for r in rows),'shortened_cases':sum(r['saved']>0 for r in rows),
              'mean_ms':sum(r['ms'] for r in rows)/100,'max_ms':max(r['ms'] for r in rows),
              'counts':{key:sum(r[key] for r in rows) for key in ('candidates','searches','expanded','state_caps','replacements','independent','changed_first_split')},
              'manual_window_original_related_moves':7,'manual_window_replacement_moves':len(replacement),
              'manual_window_same_board':True}
    (AUDIT/'diagnostic_verification.json').write_text(json.dumps({**result,'cases':rows},indent=2)+'\n')
    print(json.dumps(result,indent=2))


def evaluation():
    audit = verify_sources()
    records = [json.loads(line) for line in (ROOT/'results/eval_records.jsonl').read_text().splitlines()]
    current = [r for r in records if r['bin']==BIN]
    parent = {r['case_name']:r for r in records if r['run_id']==PARENT_RUN}
    assert len(current)==len(parent)==100 and len({r['run_id'] for r in current})==1
    cases=[]
    for record in sorted(current,key=lambda r:r['case_name']):
        assert record['status']=='ok' and record['local'] and record['input_dir']=='tools/in'
        path=ROOT/record['stdout_path'];T=verify_output(record['case_name'],path)
        counts,times,diagnostics=log_values(path.with_suffix('.txt.err'))
        assert T==record['score']==counts['T']==counts['final_ops']==counts['validated_moves']
        assert counts['E']==0 and T==counts['pre_joint_ops']-counts['joint_window_saved']
        header=re.search(r'pre_lns=(\d+) pre_pair=(\d+) pre_joint=(\d+)',path.with_suffix('.txt.err').read_text())
        pre_lns,pre_pair,pre_joint=map(int,header.groups())
        assert pre_pair-pre_joint==counts['pair_transfer_saved']
        assert pre_lns-pre_pair==counts['lns_saved']
        baseline=parent[record['case_name']]
        old_counts,_,_=log_values((ROOT/baseline['stdout_path']).with_suffix('.txt.err'))
        cases.append({**record,'parent_T':baseline['score'],'delta_T':T-baseline['score'],
                      'pre_lns_delta':pre_lns-old_counts['pre_lns_ops'],
                      'parent_attempts':old_counts['lns_attempts'],'counts':counts,'times_ms':times,'diagnostics':diagnostics})
    total=sum(r['score'] for r in cases);baseline=sum(r['score'] for r in parent.values())
    counts={key:sum(r['counts'][key] for r in cases) for key in cases[0]['counts'] if key.startswith('joint_window_')}
    errors={key:sum(r['counts'][key] for r in cases) for key in ERRORS}
    attempts=sum(r['counts']['lns_attempts'] for r in cases);old_attempts=sum(r['parent_attempts'] for r in cases)
    result={'run_id':current[0]['run_id'],'parent_run_id':PARENT_RUN,'solver_sha256':audit['solver_sha256'],
            'verified_cases':100,'total_T':total,'parent_T':baseline,'delta_T':total-baseline,
            'delta_percent':100*(total/baseline-1),'wins':sum(r['delta_T']<0 for r in cases),
            'draws':sum(r['delta_T']==0 for r in cases),'losses':sum(r['delta_T']>0 for r in cases),
            'case0000_T':cases[0]['score'],'max_elapsed_ms':max(r['elapsed'] for r in cases),'errors':errors,
            'diagnostic_count':sum(len(r['diagnostics']) for r in cases),'counts':counts,
            'mean_joint_ms':sum(r['times_ms']['joint_window'] for r in cases)/100,
            'max_joint_ms':max(r['times_ms']['joint_window'] for r in cases),
            'shortened_cases':sum(r['counts']['joint_window_saved']>0 for r in cases),
            'attempts':attempts,'parent_attempts':old_attempts,'attempts_delta_percent':100*(attempts/old_attempts-1),
            'pre_lns_delta_T':sum(r['pre_lns_delta'] for r in cases),'pre_lns_changed_cases':sum(r['pre_lns_delta']!=0 for r in cases),
            'pre_joint_delta_T':sum(r['counts']['pre_joint_ops']-r['parent_T'] for r in cases)}
    result['adopt']=not any(errors.values()) and result['diagnostic_count']==0 and result['max_elapsed_ms']<=2000 and result['case0000_T']<=43 and total<baseline and counts['joint_window_saved']>0
    with (ROOT/'adhoc/v041_comparison.csv').open('w') as file:
        writer=csv.writer(file);writer.writerow(('case','parent_T','T','delta_T','joint_saved','searches','expanded','joint_ms'))
        for r in cases:
            c=r['counts'];writer.writerow((r['case_name'],r['parent_T'],r['score'],r['delta_T'],c['joint_window_saved'],c['joint_window_searches'],c['joint_window_expanded'],r['times_ms']['joint_window']))
    (ROOT/'adhoc/v041_evaluation_summary.json').write_text(json.dumps({**result,'cases':cases},indent=2)+'\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    {'diagnostic':diagnostic,'evaluation':evaluation}[sys.argv[1]]()
