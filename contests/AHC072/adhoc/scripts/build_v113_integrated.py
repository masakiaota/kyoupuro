#!/usr/bin/env python3
"""事前固定した既存3機構の組み合わせを、v111へそのまま移す。"""
import difflib
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / 'results/nn_rank/v113/20261004_integrated_studio'
CONDITIONS = ['h', 'r', 's', 'hr', 'hs', 'rs', 'hrs', 'hrp']
PARENTS = {
    'base': 'v111_weight_portfolio.cpp',
    'common': 'v311_neural_multistart.cpp',
    'r': 'v315_reactive_racing.cpp',
    's': 'v314_conditional_support.cpp',
    'h': 'v318_bit_parallel_history.cpp',
    'hrp': 'v319_integrated_learned_racing.cpp',
}


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def transplant(target, before, after):
    """重みや行番号へ依存せず、親差分の全編集を完全一致で移植する。"""
    old, new = before.splitlines(True), after.splitlines(True)
    edits = []
    for tag, a, b, c, d in difflib.SequenceMatcher(None, old, new, autojunk=False).get_opcodes():
        if tag == 'equal' or a == 0:
            continue  # 先頭の版名・説明だけは新しい実験のヘッダにする。
        remove, insert = ''.join(old[a:b]), ''.join(new[c:d])
        if remove and target.count(remove) == 1:
            target = target.replace(remove, insert, 1)
        else:
            # 挿入位置や短い式は、変更範囲の隣の一致した文脈で特定する。
            located = False
            for extent in range(1, 17):
                for left, right in ((0, extent), (extent, 0), (extent, extent)):
                    prefix, suffix = ''.join(old[max(0, a-left):a]), ''.join(old[b:b+right])
                    needle = prefix + remove + suffix
                    if needle and target.count(needle) == 1:
                        target = target.replace(needle, prefix + insert + suffix, 1)
                        located = True
                        break
                if located:
                    break
            if not located:
                raise RuntimeError(f'移植位置を一意に確認できない: {a}:{b} {remove[:160]!r}')
        edits.append(dict(old_lines=[a+1, b], before_sha256=digest(remove), after_sha256=digest(insert)))
    return target, edits


def nn_region(text):
    return text[text.index('namespace nn {'):text.index('} // namespace nn')]


def create():
    texts = {k: (ROOT/'src/bin'/name).read_text() for k, name in PARENTS.items()}
    assert digest(texts['base']) == 'dd45427b0e3e791bf40c1a7f7560c734abde1d20452ffa6c34cd878fd8c02316'
    frozen = RUN/'frozen'; frozen.mkdir(parents=True, exist_ok=True)
    result = dict(parents={k: dict(path='src/bin/'+PARENTS[k], sha256=digest(v)) for k, v in texts.items()}, sources={})
    for label in CONDITIONS:
        if label=='hrp':
            result['sources'][label]=dict(path='src/bin/'+PARENTS[label],sha256=digest(texts[label]),edits={},upstream='37e3864')
            continue
        text = texts['base']; applied = {}
        for feature in 'rhs':
            if feature not in label:
                continue
            before = texts['r'] if feature == 'h' else texts['common']
            text, applied[feature] = transplant(text, before, texts[feature])
        if 'h' in label:
            # GCC15/ARMのv111は集計後の乗算を丸めてから減算する。
            # インライン化によるFMAへの縮約を避け、元の優先値を保つ。
            needle='            cand.priority=(saved-repair_weight*cand.broken_support_jumps+0.25)/divisor[cand.ids.size()]*noise;'
            assert text.count(needle)==1
            text=text.replace(needle,
                '            // Preserve the v111 multiplication rounding boundary on GCC15/ARM.\n'
                '            volatile double repair_penalty=repair_weight*cand.broken_support_jumps;\n'
                '            cand.priority=(saved-repair_penalty+0.25)/divisor[cand.ids.size()]*noise;')
        name = 'v113_integrated_'+label+'.cpp'
        # 古い実験の先頭説明は落とし、実際の直接親を明記する。
        body = text[text.index('#include'):]
        text = (f'// {name}\n'
                f'// v113: v111 policy and incremental CNN; fixed integration {label.upper()}.\n'
                '// H: v318 exact history summaries; R: v315 two-candidate racing;\n'
                '// S: v314 conditioned support reinsertion. Parent budgets are unchanged.\n'+body)
        assert nn_region(text) == nn_region(texts['base'])
        assert text.count('auto encode_cache=make_unique<EncodeCache>();') == 1
        assert len(text.encode()) < 512000
        path = ROOT/'adhoc/bin'/name
        if path.exists():
            assert path.read_text() == text
        else:
            path.write_text(text)
        (frozen/name).write_text(text)
        (frozen/(label+'.patch')).write_text(''.join(difflib.unified_diff(texts['base'].splitlines(True), text.splitlines(True),
                                                                fromfile=PARENTS['base'], tofile=name)))
        result['sources'][label] = dict(path=str(path.relative_to(ROOT)), sha256=digest(text), edits=applied)
    manifest = RUN/'sources.json'
    if manifest.exists():
        assert json.loads(manifest.read_text()) == result
    else:
        manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    for key, text in texts.items():
        (frozen/PARENTS[key]).write_text(text)
    if not (frozen/'preregister.md').exists():
        (frozen/'preregister.md').write_text((ROOT/'notes/experiments/v113.md').read_text())
    print(json.dumps({k: {'sha256': v['sha256'], 'edits': {f: len(e) for f, e in v['edits'].items()}}
                      for k, v in result['sources'].items()}, indent=2))


if __name__ == '__main__':
    create()
