#!/usr/bin/env python3
"""保存済みv085成功窓を集計する。探索・再学習・新しい教師生成は呼ばない。"""
from __future__ import annotations
import argparse
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
import csv
import gzip
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def scan_file(task):
    path, item, initial_ids = task
    index = item['index']; role = item['role']; path = Path(path)
    teacher = path.parents[2].name; round_name = path.parents[1].name
    buckets = {}; sizes = Counter(); rows = []
    all_ids = set(); neutral_ids = set(); success_ids = set(); immediate_neutral_ids = set(); preceding_neutral_ids = set()
    neutral_occurrences = preceding_neutral_occurrences = immediate_neutral_occurrences = occurrences = 0
    episode_count = before_best_equal = 0; prefix = None
    with gzip.open(path, 'rt') as stream:
        for episode in map(json.loads, stream):
            episode_count += 1
            run_prefix = f"{episode['teacher']}:{episode['run_seed']}:"
            if prefix is None: prefix = run_prefix
            assert prefix == run_prefix and episode['teacher'] == teacher
            transitions = episode['transitions']; assert 1 <= len(transitions) <= 64
            terminal = transitions[-1]
            assert terminal['after_T'] == episode['best_after_T'] < episode['best_before_T']
            assert terminal['before_T'] >= episode['best_before_T']
            for t in transitions:
                assert t['transition_id'].startswith(prefix)
                occurrences += 1; all_ids.add(t['transition_id'])
                if t['before_T'] == t['after_T']:
                    neutral_occurrences += 1; neutral_ids.add(t['transition_id'])
            for t in transitions[:-1]:
                if t['before_T'] == t['after_T']:
                    preceding_neutral_occurrences += 1; preceding_neutral_ids.add(t['transition_id'])
            if len(transitions) >= 2 and transitions[-2]['before_T'] == transitions[-2]['after_T']:
                immediate_neutral_occurrences += 1; immediate_neutral_ids.add(transitions[-2]['transition_id'])
            if terminal['transition_id'] in success_ids: continue
            success_ids.add(terminal['transition_id'])
            kind = terminal['kind']; selection = terminal['selection']; ids = selection['ids']; order = selection['insertion_order']
            available = bool(ids) and len(set(ids)) == len(ids) and set(ids) <= initial_ids
            order_full = available and len(order) == len(ids) and set(order) == set(ids)
            direct_gain = terminal['before_T'] - terminal['after_T']
            best_gain = episode['best_before_T'] - episode['best_after_T']
            equal = terminal['before_T'] == episode['best_before_T']; before_best_equal += equal
            key = (teacher, kind, role)
            bucket = buckets.setdefault(key, Counter())
            bucket.update(successes=1, direct_saved=direct_gain, best_saved=best_gain,
                          uphill_recovery=direct_gain-best_gain, ids_available=int(available),
                          order_available=int(order_full), before_equals_best=int(equal),
                          actual_order_deducible=int(kind in ('regular', 'dependency') and order_full and len(ids) not in (2,3,4)))
            sizes[(teacher, kind, role, len(ids) if available else -1)] += 1
            rows.append(dict(index=index, role=role, teacher=teacher, round=round_name, kind=kind,
                             episode_id=episode['episode_id'], transition_id=terminal['transition_id'],
                             before_T=terminal['before_T'], after_T=terminal['after_T'],
                             best_before_T=episode['best_before_T'], direct_saved=direct_gain, best_saved=best_gain,
                             ids_available=int(available), size=len(ids) if available else -1,
                             order_available=int(order_full), episode_length=len(transitions),
                             path=str(path.relative_to(ROOT))))
    stats = json.loads((path.parent/'teacher_stats.json').read_text())
    assert episode_count == stats['episodes_saved']
    return dict(prefix=prefix, buckets=[(*key, dict(value)) for key,value in buckets.items()],
                sizes=[(*key,value) for key,value in sizes.items()], rows=rows,
                windows=dict(episodes=episode_count, successes=len(success_ids), episodes_seen=stats['episodes_seen'],
                             saved_transition_occurrences=occurrences, saved_transition_unique=len(all_ids),
                             neutral_occurrences=neutral_occurrences, neutral_unique=len(neutral_ids),
                             preceding_neutral_occurrences=preceding_neutral_occurrences,
                             preceding_neutral_unique=len(preceding_neutral_ids),
                             immediate_preceding_neutral_occurrences=immediate_neutral_occurrences,
                             immediate_preceding_neutral_unique=len(immediate_neutral_ids),
                             success_before_equals_best=before_best_equal))


def write_csv(path, rows):
    with path.open('w', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]) if rows else [])
        writer.writeheader(); writer.writerows(rows)


def example(row, run, items):
    with gzip.open(ROOT/row['path'], 'rt') as stream:
        for episode in map(json.loads, stream):
            if episode['episode_id'] == row['episode_id']:
                before = episode['transitions'][-2]['after'] if len(episode['transitions']) > 1 else episode['start']
                return dict(**row, input_path=str((run/items[row['index']]['path']).relative_to(ROOT)),
                            before=before, terminal=episode['transitions'][-1])
    raise AssertionError('example disappeared')


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True); parser.add_argument('--workers', type=int, default=8)
    args=parser.parse_args();run=args.run.resolve();output=args.output.resolve();output.mkdir(parents=True, exist_ok=True)
    items=json.loads((run/'input_manifest.json').read_text());assert len(items)==512
    assert Counter(x['role'] for x in items)==Counter(train=384,validation=128)
    tasks=[]
    for item in items:
        tokens=(run/item['path']).read_text().split();N=int(tokens[0]);grid=tokens[2:2+N]
        cells=[c for line in grid for c in line if c!='#'];initial_ids={i for i,c in enumerate(cells) if 'a'<=c<='l'}
        paths=sorted((run/'cases'/f"{item['index']:06d}").glob('*/round_*/search/episodes.jsonl.gz'))
        assert len(paths)==6,(item['index'],len(paths))
        tasks.extend((str(path),item,initial_ids) for path in paths)
    buckets={};sizes=Counter();windows=Counter();window_groups={};rows=[];prefixes=set()
    with ProcessPoolExecutor(max_workers=args.workers) as pool:
        for i,result in enumerate(pool.map(scan_file,tasks,chunksize=2),1):
            if result['prefix'] is not None:
                assert result['prefix'] not in prefixes;prefixes.add(result['prefix'])
            for teacher,kind,role,bucket in result['buckets']:
                buckets.setdefault((teacher,kind,role),Counter()).update(bucket)
            for teacher,kind,role,size,count in result['sizes']:sizes[(teacher,kind,role,size)]+=count
            windows.update(result['windows']);rows.extend(result['rows'])
            task=tasks[i-1];group=(Path(task[0]).parents[2].name,task[1]['role'])
            window_groups.setdefault(group,Counter()).update(result['windows'])
            if i%256==0:print(f'{i}/{len(tasks)} files',flush=True)
    rows.sort(key=lambda r:(r['index'],r['teacher'],r['round'],r['episode_id']))
    by_kind=[dict(teacher=k[0],kind=k[1],role=k[2],**v) for k,v in sorted(buckets.items())]
    by_size=[dict(teacher=k[0],kind=k[1],role=k[2],size=k[3],count=v) for k,v in sorted(sizes.items())]
    write_csv(output/'success_actions.csv',rows);write_csv(output/'by_kind_split.csv',by_kind);write_csv(output/'set_sizes.csv',by_size)
    selected=[]
    for predicate in [lambda r:r['teacher']=='v801' and r['kind']=='regular' and r['size'] in (2,3,4),
                      lambda r:r['teacher']=='v801' and r['kind'] in ('regular','dependency') and r['size']>4,
                      lambda r:r['teacher']=='v801' and r['kind']=='paired']:
        selected.append(next(r for r in rows if r['role']=='train' and predicate(r)))
    examples=[example(row,run,items) for row in selected]
    (output/'examples.json').write_text(json.dumps(examples,ensure_ascii=False,indent=2)+'\n')
    summary=dict(run=str(run.relative_to(ROOT)),inputs=dict(total=512,train=384,validation=128),files=len(tasks),
                 windows=dict(windows),window_groups=[dict(teacher=k[0],role=k[1],**v) for k,v in sorted(window_groups.items())],
                 by_kind_split=by_kind,set_sizes=by_size,examples=[{k:v for k,v in ex.items() if k not in ('before','terminal')} for ex in examples],
                 interpretation={'direct_saved':'terminal.before_T - terminal.after_T',
                                 'best_saved':'episode.best_before_T - episode.best_after_T',
                                 'window':'at most 64 accepted changed transitions including success; at most 63 predecessors',
                                 'neutral':'before_T==after_T with changed move sequence; deduplicated only by transition_id',
                                 'sampling':'retained success windows, capped reservoir 64 per C++ call; not all improvements or all accepted moves'})
    (output/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(dict(files=len(tasks),**windows),ensure_ascii=False))

if __name__=='__main__':main()
