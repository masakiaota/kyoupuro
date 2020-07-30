# https://atcoder.jp/contests/arc038/tasks/arc038_b
# なぜ後ろからDPまでわかっててなぜそのままACまでいけなかったのか... 先行後攻で頭バグらせすぎ

# i,jからゲームスタートしたときに先攻が勝つか負けるか考えてみる
# 障害物や場外の場合は先攻が必ず勝つ(その前に後攻が場外にはみ出し負けているので)
# 自分から見て、右、右下、下に(負け)のマスがある場合は、そのマスに移動するのが最適。(相手に負けを押し付けられる)。よって自分は勝ちとなる。
# 逆に自分から見て右、右下、下に(負け)のマスがない場合は、どのように移動しても相手の勝ち状態になってしまうので自分の負け

import sys
from functools import lru_cache


sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def ints(): return list(map(int, read().split()))


def read_map_as(H, replace={'#': 1, '.': 0}, pad=None):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    if pad is None:
        ret = []
        for _ in range(H):
            ret.append([replace[s] for s in read()[:-1]])
            # 内包表記はpypyでは若干遅いことに注意
            # #numpy使うだろうからこれを残しておくけど
    else:  # paddingする
        ret = [[pad] * (W + 2)]  # Wはどっかで定義しておくことに注意
        for _ in range(H):
            ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        ret.append([pad] * (W + 2))

    return ret


H, W = ints()
S = read_map_as(H, pad=1)


@lru_cache(maxsize=2**12)
def is_F(i, j)->bool:  # i,jにいるときにFが勝つか
    if S[i][j]:  # 障害物or場外
        return True
    return False in [is_F(i, j + 1), is_F(i + 1, j + 1), is_F(i + 1, j)]


print(['Second', 'First'][is_F(1, 1)])
