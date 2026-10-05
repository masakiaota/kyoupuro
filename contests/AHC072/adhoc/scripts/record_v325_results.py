#!/usr/bin/env python3
"""Record completed v325 results only; never run solvers, learn or register evals."""
from pathlib import Path
import csv
import hashlib
import io
import json

ROOT = Path(__file__).resolve().parents[2]
D = ROOT / 'adhoc/v325'
SOURCE_SHA = '1dce487ed293ba3e8cd44cf93ecddc8ed0e8b5cb3ac6f42d48ff15fd50769141'
PAYLOAD_SHA = 'c465382cb8f38a2b5c30bb2cceca55fd662ac0dec0fcbda33a3645ade9282884'
SUMMARY_SHA = '3b7379277d0dbfe01d7bbdf166894573aaa3a0c52a1442677397a5054f6c6d91'
CSV_SHA = 'cb4c2d962cd2103f7ae6981aba7ee00269b463cf335af6661a5ade1726d5b102'

def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def main() -> None:
    if sha((ROOT / 'src/bin/v325_exact_hotpath.cpp').read_bytes()) != SOURCE_SHA:
        raise RuntimeError('Frozen source mismatch')
    data = (D / 'score_payload.json').read_bytes()
    if sha(data) != PAYLOAD_SHA:
        raise RuntimeError('Score transport mismatch')
    payload = json.loads(data)
    summary = json.loads((D / 'evaluation_summary.json').read_text())
    canonical = json.dumps(summary, sort_keys=True, separators=(',', ':')).encode()
    if sha(canonical) != SUMMARY_SHA:
        raise RuntimeError('Summary transport mismatch')
    stream = io.StringIO(newline='')
    writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(['set', 'case', 'v113', 'v325', 'v800_reference', 'delta'])
    for name, expected_count in [('fresh128', 128), ('in', 100)]:
        pairs = payload[name]
        if len(pairs) != expected_count:
            raise RuntimeError('Input count mismatch')
        for j, version in enumerate(('v113', 'v325')):
            if sum(row[j] for row in pairs) != summary['sets'][name][version]['sum']:
                raise RuntimeError('Score sum mismatch')
        for i, (base, new) in enumerate(pairs):
            reference = payload['v800_reference'][i] if name == 'in' else ''
            writer.writerow([name, f'{i:04d}.txt', base, new, reference, new-base])
    result = stream.getvalue().encode()
    if sha(result) != CSV_SHA:
        raise RuntimeError('Final CSV checksum mismatch')
    (D / 'paired_scores.csv').write_bytes(result)
    note = ROOT / 'notes/experiments/v325.md'
    original = note.read_text()
    marker = '<!-- v325 completed evidence -->'
    if marker not in original:
        note.write_text(original.rstrip() + '\n\n' + marker + '\n\n' + (D / 'completed_note.md').read_text().rstrip() + '\n')
    backlog = ROOT / 'notes/backlog.md'
    text = backlog.read_text()
    entry = '- **[v325] 同じ探索の計算とメモリ転送を削る** → [v325](experiments/v325.md) 高速化候補。v113を対照に、既成v136とNN投影再利用、差分ハッシュ、二端点退避、存在する隙間だけの転送、束遷移表キャッシュ、除算省略、開番地辞書、出力方向の並列演算、限定した自動ゼロ初期化抑止を統合。固定16入力の提出相当CPU時間は26.71%減、v136比6.16%減。全操作列と乱数と時計照会数の64比較、768部分問題、422960 logitsの照合を通過。非LOCAL1.9秒のinは14手減、v800相対+0.057580、新規128は59手減。ただしin0005を除く99件は5手増。通常の除去と再挿入の部分計測はほぼ同じ。456出力が公式採点と独立再生で合法かつ全帰巣、全件1.9秒以内、新版最大1.883860秒。GCC14.2/x86の単回比較でAtCoder実機の倍率は未保証。validation1とeval viewerは未使用。現行指定はv113を保持する。\n'
    if '**[v325]' not in text:
        heading = '## 決着済み\n'
        if heading not in text:
            raise RuntimeError('Backlog section changed; preserve existing content')
        backlog.write_text(text.replace(heading, heading + '\n' + entry, 1))
    print('Verified completed v325: source, summary, 228 paired scores; no solver execution')

if __name__ == '__main__':
    main()
