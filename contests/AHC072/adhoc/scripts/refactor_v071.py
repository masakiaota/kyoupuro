#!/usr/bin/env python3
"""Apply the registered, behavior-preserving source cleanup to frozen v059."""
from pathlib import Path
import difflib
import json
import re

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / 'src/bin/v059_repair_priority.cpp'
CHILD = ROOT / 'src/bin/v071_refactored.cpp'
OUT = ROOT / 'adhoc/v071_audit'
# Match literals and comments before identifiers; never rename inside either.
LEX = re.compile(r'//[^\n]*|/\*[\s\S]*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|\b([A-Za-z_]\w*)\b')

COMMENTS = {
1: 'v071_refactored.cpp\n探索の実数演算の順序を保つためfast-mathは使わない。',
11: '', 16: 'LOCALでは時間を補正し、集計を有効にする。',
64: '頻出処理では整数を加算し、文字列への変換と出力は探索後に行う。',
87: '開始時刻の追加を、束のまま・個体へ分解・2塔の再構築に分けて集計する。',
135: '失敗と期限到達による中断も、追加順序の費用へ含める。',
144: '不採用と期限到達による中断も、依存拡張の費用へ含める。',
159: '',
165: '盤面は床数分の領域を借りる。2塔の再構築3枚と再挿入1枚で最大4枚を同時に使う。',
186: '引数なしでは領域だけ取得する。利用前にIdentityState::wordsで全床を書き込む。',
235: '64 bit積の上位を使い、整数除算なしで[0,n)へ写す。',
262: '単位費用の探索用。費用別の連結リストを使い、古い候補の検査は呼び出し側で行う。',
343: '帰巣する前の着地容量を含め、操作の合法性を検査する。',
373: '空盤面で帰巣する費用。色の連続部分の境界で分割し、塔の両方向を評価する。',
450: '周囲の塔を固定した束の最短輸送。着地先の塔を残して再出発でき、その高さを飛距離に使う。',
495: '1束の全分割位置を評価する。上側を帰巣させてから下側を扱うため、未処理部分の容量を保てる。',
602: '分割か帰巣で構築へ制御を戻し、残った束に別の色を合流させる機会を作る。',
625: '期限付近の1匹ずつの帰巣。高さ8の塔を先に空け、通過先の容量を確保する。',
661: '色ごとの巣へ増分的に木をつなぎ、容量を守って葉から集める初期解。',
717: '異色の下段を踏み台にし、集荷木の空いた直線部分を飛び越す。',
889: '周囲の塔を合流させると分割DPの利得を失う場合は、元の束の帰巣まで完了させる。',
905: '近い束を踏み台に残せるよう、遠い束から帰巣させる。',
978: '塔の上側だけを運ぶ最短経路。初回の出発で消える巣上の下段も扱う。',
1024: '連続した輸送を、終了時の全盤面が等しい短い経路へ置き換える。',
1063: '帰巣のない2マス間の転送では、一方を下から、他方を上から連結した列が不変。\n両端へ触れない操作をまたいで正味の転送をまとめ、最初の転送位置へ置く。',
1080: '着地点を使う前に、元の操作の合法性を検査する。',
1093: '元の最後の出発で下段がl-1匹以上残るため、正味の向きが反転しても直接転送できる。',
1102: '相殺された往復より前の最終接触を戻し、さらに前の転送とも相殺できるようにする。',
1107: '不変条件の破れは、入力をそのまま返して隠さず例外にする。',
1134: '区間の入口と出口だけを固定し、途中の分割・合流を探索する。\n各塔は8匹まで。参加総数の増加は状態数上限と期限で抑える。',
1200: '逆探索では順方向に合法な出発の直前を列挙する。物理的な逆ジャンプの支持は要求しない。',
1350: '省略した操作の接触先はblockedで窓への追加を禁止したため、置換全体と交換できる。',
1369: '',
1381: '同色のp→q、p→r、r→qを、踏み台部分もまとめて輸送する。\np/q/rへ触れない操作は交換し、それ以外の接触で区間を閉じる。',
1402: '置換を最初の出発へ移すため、中継塔に変更がないことを積み順まで確認する。',
1433: '容量は帰巣前に検査する。全盤面の一致で、踏み台や中継巣の帰巣も確認する。',
1497: '合流・分割・帰巣までを1輸送として読む。次の輸送では束の構成が変わり得る。',
1514: '区間境界で高さを保ち、最大2塔の積み順の差を後の輸送まで引き継ぐ。',
1537: '上下対称な束には、後の輸送まで引き継ぐ向きの差がない。',
1540: '厳密側は最初から短縮する経路を残す。緩和側は1手の超過を許し、候補4本で追跡する。',
1576: '無関係な操作は順序を保って再生し、経路を探索し直さない。',
1588: '先の短縮を後の輸送に使えるよう、手数上限は区間全体へ適用する。',
1642: '置換後の全盤面が一致しているため、期限到達時は残りの操作をそのままつなげられる。',
1657: '1回のBFSで全着地点を調べる。帰巣が起きた着地で輸送を切り、変化後の盤面から再探索する。',
1696: '固定背景上の全可動塔をマスと色順で表す。個体IDは区別せず、積み順と容量を保つ。',
1824: '帰巣した個体は戻せないため、出口で必要な匹数を下回った状態を捨てる。',
1829: '枝刈りは下界だけで行う。距離と積み順は、出口に塔を残す場合も含めた探索順位に使う。',
1962: '1回の探索予算を全体の期限内に収める。無限の期限は固定仕事量の診断だけで使う。',
2038: '短縮で分割・接触順が変わるため、各巡で短くなる間は再適用する。\n有限区間の探索後にも安い短縮を行えるよう、終了予算を少し残す。',
2068: '期限到達までに完了した短縮は残す。その他の例外は呼び出し側へ伝える。',
2073: '',
2075: '受理判定前に候補を短縮する。変更付近への重点化は選択基準であり、境界一致の保証ではない。',
2136: 'LNS経過時間の一定割合を予算とし、開始直後にも少量の余裕を与える。',
2189: '',
2191: '個体IDは初期位置の床ID。同色の束の除去では、色順が同じなら個体IDの割り当てを交換できる。',
2232: '候補の索引を再利用する。世代番号で初期化し、候補ごとの要素確保・解放を避ける。',
2265: '同じ予定時刻の空間探索。費用の小さい順に確定し、同費用では補助評価だけを更新する。',
2271: '上位を世代、下位を待機費用に使う。各時刻の初期化を費用桶の数に抑える。',
2306: '下段の個体は複数の便を支え得る。slack匹までなら失ってもそのジャンプは成立する。',
2331: '支持不足になる残存便を数える。修復負担の代理値であり、追加手数の下界ではない。',
2403: '',
2429: '個体の除去で壊れたジャンプを修復し、残存便の色順を保つ。露出した巣上の個体は先に帰巣できる。',
2467: '同色の個体を交換し、後の分割で使うIDの割り当てを保つ。',
2483: '同費用なら、次の同色2便を1操作へまとめられる支持を優先する。\n短縮後に操作の合法性と境界の全盤面一致を検査する。',
2496: '固定した予定への1匹の再挿入。状態はマスと塔内の隙間で、追加移動は費用1、予定操作は費用0。\n同費用では同乗と踏み台の利用を優先する。',
2507: '初期の隙間を指定し、任意の挿入順から元の塔の積み順を復元する。',
2562: '追加個体が帰巣へ影響するのは、残る背景の最上段以上にいる場合だけ。\n位置を式で求め、各隙間での塔の再構築を省く。容量は帰巣前の高さで判定する。',
2587: '背景の帰巣を止める異色の追加個体は、即座に出発すれば予定を復元でき、下段も支持に使える。',
2611: '変更された2マスとそこへ入る辺を開き直す。巣の帰巣で最上段の状態が悪化した場合は、\n高さが同じでも再探索が必要。他の状態はそのまま待機できる。',
2635: '第2順序では先頭2匹だけを交換する。後続の個体は、それぞれで作られた経路へ挿入する。',
2653: '挿入では操作数が増えるため、完成済みの解より短くならない第2順序を枝刈りする。',
2678: '連続した束を一単位で再挿入する。状態は上下反転と最上段の帰巣で到達する列だけに絞る。',
2771: '束の上に背景の異色があれば帰巣を止める。なければ束の最上段の同色部分だけを帰す。',
2851: '途中の塔の上側を除去し、個体ごとに再挿入する。元の段から隙間を決め、開始盤面を復元する。\n挿入後の個体は分離・同乗・再合流できる。',
2885: '巣では帰巣しない元の最上段を先に置き、構築途中の帰巣を防ぐ。',
2913: '同じ途中時点の2塔の上側を再構築する。各個体の出発点と元の段を保持し、\n開始盤面と既存の前半操作を保ったまま、その後の経路を作り直す。',
2929: '既に行った集荷を残しつつ、早い時点を優先する。',
3016: '巣では元の最上段を先に置き、他の個体をその下に挿入して帰巣を防ぐ。',
3049: '同じ端点へ触れる操作の順序を保つ。他の操作は交換できるため、\nトポロジカル順序の変更で便をつなぎ、後の再挿入の同乗・支持機会を変える。',
3148: '開始時刻ごとに異なるキーを使い、別時刻の試行による再試行待ちを共有させない。',
3157: '各操作から最大8色列、便、下段、着地先との和集合を取り、空間・単独・同色候補を加える。',
3215: '塔全体、跳ぶ束、上側2匹を候補にする。再挿入時には個体へ分解できる。',
3232: '同じ個体集合に開始時刻を1枠追加する。分離する境界を優先し、同条件なら遅い時刻を選ぶ。',
3240: '同じ端点で相互作用した束の和集合を、位置の近さだけによらない除去候補にする。',
3250: '最初の候補群を集めてから追加し、候補順と優先度の乱数消費順を固定する。',
3264: '各集合を最小IDへ登録する。除去対象に含まれる最小IDの集合だけを調べ、\n操作時刻の一覧から区間内の削除費用を求める。',
3285: '候補ごとに乱数を1回引く。負の優先度も候補として保持する。',
3323: '最初の並べ替えは短くなった場合に採用し、同手数の別順序は後の探索で扱う。',
3351: '支持の重みは近傍の種類と独立に抽選する。',
3364: '通常の除去対象の選択4回に1回を、最大8匹の依存拡張へ割り当てる。',
3481: '経路短縮は操作を再生し、変更区間の全盤面一致と最後の全帰巣を確認する。',
3558: '個体IDと可変長の塔で独立再生し、ビット表現と再挿入器から独立に検証する。',
3671: '線形走査の転送圧縮を行い、探索の短縮と分けて集計する。',
3679: '残った終了処理の時間で共同短縮する。探索途中で期限に達した区間は置換しない。',
3690: '', 3700: '',
}


def identifiers(source):
    return {m[1] for m in LEX.finditer(source) if m[1]}


def rename(source, mapping):
    return LEX.sub(lambda m: mapping.get(m[1], m[0]) if m[1] else m[0], source)


def prepare():
    source = PARENT.read_text()
    changes = []

    def replace(old, new, reason):
        nonlocal source
        count = source.count(old)
        assert count, reason
        changes.append({'old': old, 'new': new, 'count': count, 'reason': reason})
        source = source.replace(old, new)

    # Comment ranges are identified against the unchanged parent, before edits.
    lines = source.splitlines(True)
    for start, content in COMMENTS.items():
        i = start - 1
        assert lines[i].lstrip().startswith('//'), start
        end = i + 1
        while end < len(lines) and lines[end].lstrip().startswith('//'):
            end += 1
        indent = re.match(r'\s*', lines[i]).group()
        new = ''.join(indent + '// ' + text + '\n' for text in content.split('\n')) if content else ''
        replace(''.join(lines[i:end]), new, 'comment')

    for old in dict.fromkeys(re.findall(r'^#ifdef AHC072_\w+_AUDIT\n.*?^#endif\n', source, re.M | re.S)):
        assert '#if' not in old[len(old.splitlines()[0]) + 1:]
        replace(old, '', 'inactive audit hook')
    replace('    int evaluate(Word s,int p) { return value(get(s),p); }\n', '', 'unused function')
    replace('    static Word eraseToken(Word w,int gap) {\n        Word upper=gap==7?0:w>>(4*(gap+1));\n        return prefix(w,gap)|(upper<<(4*gap));\n    }\n', '', 'unused function')
    replace('JointWindowStats& stats,vector<JointPatch>* audit=nullptr,',
            'JointWindowStats& stats,', 'unused audit output parameter')
    replace('vector<FinitePatch>* audit=nullptr,int first_begin=0',
            'int first_begin=0', 'unused audit output parameter')
    replace('    if(audit)*audit=move(patches);\n', '', 'unused audit output branch')
    replace('answer,deadline,joint,nullptr,focus.first,focus.last',
            'answer,deadline,joint,focus.first,focus.last', 'unused null audit argument')
    replace('answer,deadline,finite,nullptr,focus.first,focus.last',
            'answer,deadline,finite,focus.first,focus.last', 'unused null audit argument')

    # Make the non-LOCAL forms match the template. The three invocation bodies
    # are unchanged, including the order and number of clock calls under LOCAL.
    replace('#define LOCAL_NOTE(...)\n#define LOCAL_SECONDS(value) value',
            '#define LOCAL_NOTE(...)\n#define LOCAL_ONLY(...) do { } while (false)\n#define LOCAL_TIME(trace, key, ...) (__VA_ARGS__)()\n#define LOCAL_SECONDS(value) value', 'template macros')
    for old in re.findall(r'#ifdef LOCAL\n(?:    best=|    LOCAL_TIME).*?\n#else\n.*?\n#endif\n', source, re.S):
        assert old.count('#ifdef') == 1
        replace(old, old.split('\n#else\n')[0].split('\n', 1)[1] + '\n', 'shared timed call')

    begin = source.index('struct Clock {')
    end = source.index('} clk;', begin) + len('} clk;')
    old = source[begin:end]
    replace(old, rename(old, {'limit': 'time_limit_sec', 'start': 'start_'}), 'timer member names')
    while 'clk.limit' in source:
        line = next(s for s in source.splitlines(True) if 'clk.limit' in s)
        replace(line, line.replace('clk.limit', 'clk.time_limit_sec'), 'timer member use')
    replace('    clk.start = chrono::steady_clock::now();',
            '    clk.start_ = chrono::steady_clock::now();', 'timer start use')

    for old, new in (('board_payload_bytes', 'state_payload_bytes'),
                     ('board_handle_bytes', 'state_handle_bytes'),
                     ('board_pool_slots', 'state_slots'),
                     ('board_pool_bytes', 'state_pool_bytes'),
                     ('board_pool_free_at_end', 'state_pool_free_at_end')):
        replace('"'+old+'"', '"'+new+'"', 'state log key')

    # Scope-specific names with a different meaning elsewhere in the source.
    begin = source.index('struct Geometry {')
    end = source.index('} geo;', begin) + len('} geo;')
    old = source[begin:end]
    new = rename(old, {'id': 'cell_id', 'dist': 'floor_dist', 'c': 'input_char'})
    # normalize() uses a packed color code, not an input character.
    marker = new.index('    Word normalize(')
    new = new[:marker] + rename(new[marker:], {'input_char': 'code'})
    replace(old, new, 'board member names')
    source_uses = ('geo.id', 'geo.dist')
    for old, new in zip(source_uses, ('geo.cell_id', 'geo.floor_dist')):
        while old in source:
            # Record a whole line, so every edit has a unique reversible range.
            line = next(s for s in source.splitlines(True) if old in s)
            replace(line, line.replace(old, new), 'board member use')

    begin = source.index('struct World {')
    end = source.index('\n};', begin) + len('\n};')
    old = source[begin:end]
    replace(old, rename(old, {'b': 'state', 'out': 'moves', 'remaining': 'E'}), 'construction state names')
    for pattern, new in ((r'\b(w|trial)\.b\b', r'\1.state'),
                         (r'\b(w|trial|result)\.out\b', r'\1.moves'),
                         (r'\b(w|result)\.remaining\b', r'\1.E')):
        while re.search(pattern, source):
            line = next(s for s in source.splitlines(True) if re.search(pattern, s))
            replace(line, re.sub(pattern, new, line), 'construction state use')

    # Pure identifier renames, with unique destinations for reversibility.
    mapping = {'Board': 'State', 'Geometry': 'Board', 'BoardPool': 'StatePool',
               'World': 'ConstructionState', 'IdentityBoard': 'IdentityState',
               'Word': 'TowerBits', 'MV': 'max_cells', 'INF': 'infinite_cost',
               'MASK': 'tower_mask', 'geo': 'board_info', 'board_pool': 'state_pool',
               'grid': 'C', 'V': 'cell_count', 'nest': 'nest_code', 'home': 'nest_pos_by_code',
               'Clock': 'TimeKeeper', 'clk': 'time_keeper', 'RNG': 'Rng',
               'elapsed': 'exact_elapsed_sec', 'c': 'code', 'color': 'color_code',
               'reverseWord': 'reverse_tower', 'joinWord': 'append_tower',
               'topColor': 'top_color_code', 'monoColor': 'mono_color_code'}
    # 'code' was introduced in the scoped rewrite, so preserve it on reversal.
    mapping['c'] = 'current_code'
    words = identifiers(source)
    for name in sorted(words):
        if name not in mapping and name[0].islower() and re.search('[a-z][A-Z]', name):
            target = re.sub('([a-z0-9])([A-Z])', r'\1_\2', name).lower()
            if target in words or target in mapping.values():
                target += '_value'
            mapping[name] = target
    assert len(set(mapping.values())) == len(mapping)
    assert all(new not in words or new in mapping for new in mapping.values())
    renamed = rename(source, mapping)
    assert rename(renamed, {v: k for k, v in mapping.items()}) == source
    child = re.sub(r'\n(?:[ \t]*\n){2,}', '\n\n', renamed)
    # Make the representation differences explicit at their definitions.
    child = child.replace('using TowerBits = uint32_t;',
        '// 塔は下から4 bitずつ色c+1を詰める。空は0。色符号は1..K、巣なしは0。\nusing TowerBits = uint32_t;')
    child = child.replace('struct Move {',
        '// pは床ID、dはU/D/L/Rの0..3。地形のrayは飛距離lをそのまま添字にする。\nstruct Move {', 1)
    child = child.replace('struct Board {',
        '// 地形と初期入力。nest_codeは色符号、nest_pos_by_codeは符号から巣への逆引き。\nstruct Board {', 1)
    child = child.replace('struct TimeKeeper {',
        '// 非LOCALは起動時、LOCALはmain先頭から計時する。各照会で時計を読む。\nstruct TimeKeeper {', 1)
    child = child.replace('    int grow_dependency(',
        '    // 壊れる便の乗客を除去集合へ追加し、最大8匹まで依存を追う。\n    int grow_dependency(', 1)
    return child, changes, mapping


if __name__ == '__main__':
    assert not (OUT / 'frozen.json').exists(), 'Already executed: source is frozen.'
    OUT.mkdir(exist_ok=True)
    source, changes, mapping = prepare()
    CHILD.write_text(source)
    (OUT / 'registered_changes.json').write_text(json.dumps({'edits': changes, 'renames': mapping}, ensure_ascii=False, indent=2) + '\n')
    (OUT / 'source.diff').write_text(''.join(difflib.unified_diff(PARENT.read_text().splitlines(True), source.splitlines(True), fromfile=PARENT.name, tofile=CHILD.name)))
    print({'before_lines': len(PARENT.read_text().splitlines()), 'after_lines': len(source.splitlines()), 'renames': len(mapping), 'registered_edits': len(changes)})
