# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
    return ret


N, P = read_ints()
S = read()[:-1]

# すべてを見ていくのは無理
# ある連続する文字列に対して、一つ数字が増えたときの判別はどうなるのか？ よくわからない
# 倍数を探したい...
# ある倍数について、それを構成する最小の部分列を保持することを考える
# その部分列を上位桁にずらしてもそれはPの倍数であることが保証されている。つまり、連続する区間の数をかけて行くのが正解？
# 0はvalidな文字列についたら万能 それ以外はちゃんと検証する


# 次の個数=今までの個数+自身までの区間の個数
tmp = ''  # 連続する区間
cnt = 0
ans = 0  # いままでの個数
pre_is_valid = 0  # 前が有効部分文字列だったか
pre_is_0 = 0
for s in S:
    tmp += s
    if pre_is_valid and s == '0':
        # 万能なので追加する
        cnt += 1
        ans += cnt
        pre_is_0 = 1

    else:
        if int(tmp) % P == 0:
            pre_is_valid = 1
            cnt += 1
            ans += cnt
            tmp = ''
        else:
            pre_is_valid = 0
        pre_is_0 = 0

print(ans)

# プログラムは正しく動いてる
# 見落としの通りが存在する

# 既知の見落とし
# 0が連続する場合→preが0のときは
# なぜか数字が足りない
