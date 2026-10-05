from collections import defaultdict
from pathlib import Path
import csv
import json
import statistics as st

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'adhoc/v030_state_capacity'
METHODS = ('fixed32', 'exact', 'fixed400')
raw = list(csv.DictReader((DEST / 'samples.csv').open()))
for r in raw:
    for k in ('floors', 'capacity', 'pool', 'moves', 'rep', 'rank', 'bytes', 'pool_bytes', 'iterations', 'cpu_ns', 'wall_ns', 'checksum', 'base_mod128'):
        r[k] = int(r[k])
    r['ns'] = r['cpu_ns'] / r['iterations']
assert len(raw) == 18414, len(raw)
checks = defaultdict(list)
for r in raw:
    checks[(r['kind'], r['case'], r['pool'], r['moves'], r['rep'])].append(r['checksum'])
assert all(len(v) == 3 and len(set(v)) == 1 for v in checks.values())

case_groups = defaultdict(list)
for r in raw:
    case_groups[(r['kind'], r['case'], r['floors'], r['capacity'], r['pool'], r['moves'], r['method'])].append(r['ns'])
assert all(len(v) == 6 for v in case_groups.values())
per_case = {}
for key, values in case_groups.items():
    base, method = key[:-1], key[-1]
    per_case.setdefault(base, dict(zip(('kind','case','floors','capacity','pool','moves'), base)))[method] = st.median(values)
case_rows = []
for r in sorted(per_case.values(), key=lambda x:(x['kind'],x['floors'],x['case'],x['pool'],x['moves'])):
    r['exact_vs_fixed_pct'] = 100 * (r['exact'] / r['fixed32'] - 1)
    r['fixed_vs_400_pct'] = 100 * (r['fixed32'] / r['fixed400'] - 1)
    r['exact_vs_400_pct'] = 100 * (r['exact'] / r['fixed400'] - 1)
    case_rows.append(r)
with (DEST / 'per_case.csv').open('w') as out:
    writer = csv.DictWriter(out, fieldnames=list(case_rows[0])); writer.writeheader(); writer.writerows(case_rows)

def aggregate(rows):
    grouped = defaultdict(list)
    for r in rows:
        grouped[(r['rep'], r['method'])].append(r['ns'])
    averages = {m:[st.mean(grouped[(i,m)]) for i in range(6)] for m in METHODS}
    medians = {m:st.median(v) for m,v in averages.items()}
    delta = 100 * (medians['exact'] / medians['fixed32'] - 1)
    exact_wins = sum(e < f for e,f in zip(averages['exact'], averages['fixed32']))
    verdict = 'exact' if delta <= -3 and exact_wins >= 5 else 'fixed32' if delta >= 3 and exact_wins <= 1 else 'similar'
    return {'cases':len(set(r['case'] for r in rows)), 'floors_min':min(r['floors'] for r in rows), 'floors_max':max(r['floors'] for r in rows),
            'ns':medians, 'rep_ns':averages, 'exact_vs_fixed_pct':delta, 'exact_faster_reps':exact_wins, 'verdict':verdict,
            'fixed_vs_400_pct':100*(medians['fixed32']/medians['fixed400']-1), 'exact_vs_400_pct':100*(medians['exact']/medians['fixed400']-1)}

conditions = []
for kind in ('real','handmade','boundary'):
    for pool in (1,64,8192):
        for moves in ((0,) if kind == 'boundary' else (0,1,4)):
            selected = [r for r in raw if r['kind']==kind and r['pool']==pool and r['moves']==moves]
            item = {'kind':kind,'pool':pool,'moves':moves, **aggregate(selected)}
            local_cases=[r for r in case_rows if r['kind']==kind and r['pool']==pool and r['moves']==moves]
            item['exact_faster_cases'] = sum(r['exact'] < r['fixed32'] for r in local_cases)
            conditions.append(item)
by_capacity = []
for cap in sorted(set(r['capacity'] for r in raw if r['kind']=='real')):
    for pool in (1,64,8192):
        for moves in (0,1,4):
            selected = [r for r in raw if r['kind']=='real' and r['capacity']==cap and r['pool']==pool and r['moves']==moves]
            by_capacity.append({'capacity':cap,'pool':pool,'moves':moves, **aggregate(selected)})

real_cases=list(csv.DictReader((DEST/'cases.csv').open()))
real_cases=[r for r in real_cases if r['kind']=='real']
ratios=[r['wall_ns']/r['cpu_ns'] for r in raw]
summary={
    'conditions':conditions, 'by_capacity':by_capacity,
    'case_conditions':len(per_case), 'measurement_intervals':len(raw),
    'measured_clones':sum(r['iterations'] for r in raw), 'cpu_seconds':sum(r['cpu_ns'] for r in raw)/1e9,
    'wall_cpu_ratio_median':st.median(ratios), 'wall_cpu_ratio_max':max(ratios),
    'wall_cpu_ratio_over_1_1':sum(x>1.1 for x in ratios),
    'avg_bytes_fixed32':st.mean(int(r['capacity'])*4 for r in real_cases),
    'avg_bytes_exact':st.mean(int(r['floors'])*4 for r in real_cases),
}
(DEST/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
lines=['# Stateのコピー容量の測定結果','',
       'CPU時間。各ケースを同じ回数処理し、ケース等重みの平均を各区間で求め、その6区間の中央値を示す。',
       'ぴったり配列の差は32刻み固定配列に対する時間増減で、負ならぴったり配列が速い。','',
       '## 通常99ケース','',
       '| 保存先の状態数 | コピー後の操作数 | 固定32刻み ns | ぴったり ns | 固定400枠 ns | ぴったりの差 | ぴったりが速い区間 | 判定 |',
       '|---:|---:|---:|---:|---:|---:|---:|---|']
for r in conditions:
    if r['kind']!='real': continue
    n=r['ns']; verdict={'exact':'ぴったり','fixed32':'32刻み固定','similar':'同程度'}[r['verdict']]
    lines.append(f"| {r['pool']} | {r['moves']} | {n['fixed32']:.3f} | {n['exact']:.3f} | {n['fixed400']:.3f} | {r['exact_vs_fixed_pct']:+.2f}% | {r['exact_faster_reps']}/6 | {verdict} |")
lines += ['', '## 床数別：コピー＋1手', '', '| 床数 | ケース数 | 保存先 | 固定32刻み ns | ぴったり ns | ぴったりの差 |', '|---|---:|---:|---:|---:|---:|']
for r in by_capacity:
    if r['moves']!=1: continue
    lines.append(f"| {r['floors_min']}〜{r['floors_max']} | {r['cases']} | {r['pool']} | {r['ns']['fixed32']:.3f} | {r['ns']['exact']:.3f} | {r['exact_vs_fixed_pct']:+.2f}% |")
lines += ['', '## 境界サイズ：コピーのみ', '', '| 床数 | 固定容量 | 保存先 | 固定32刻み ns | ぴったり ns | ぴったりの差 |', '|---:|---:|---:|---:|---:|---:|']
for r in case_rows:
    if r['kind']!='boundary': continue
    lines.append(f"| {r['floors']} | {r['capacity']} | {r['pool']} | {r['fixed32']:.3f} | {r['exact']:.3f} | {r['exact_vs_fixed_pct']:+.2f}% |")
lines += ['', '## 集計情報', '',
          f"- 計測区間: {len(raw):,}、計測内のコピー: {summary['measured_clones']:,}回。", 
          f"- 計測内のCPU時間合計: {summary['cpu_seconds']:.3f}秒。",
          f"- 経過時間 / CPU時間: 中央値{summary['wall_cpu_ratio_median']:.3f}、最大{summary['wall_cpu_ratio_max']:.3f}。1.1超は{summary['wall_cpu_ratio_over_1_1']}区間。削除せず集計した。",
          f"- 通常99件の平均保存量: 固定32刻み{summary['avg_bytes_fixed32']:.2f} byte、ぴったり{summary['avg_bytes_exact']:.2f} byte。", '']
(DEST/'summary.md').write_text('\n'.join(lines))
print('\n'.join(lines[:18]))
print(json.dumps({k:v for k,v in summary.items() if k not in ('conditions','by_capacity')},indent=2))
