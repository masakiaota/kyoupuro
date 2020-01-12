# https://atcoder.jp/contests/abc115/tasks/abc115_d
# 真面目に再帰関数によって文字列を生成すると、長さが爆発してしまう
# 問題の再帰構造を観察すると、二分探索のような処理でPの数を確定できる


def ret_length(L):
    ret = 1
    for _ in range(L):
        ret = 2 * ret + 3
    return ret


def ret_nP(L):
    ret = 1
    for _ in range(L):
        ret = 2 * ret + 1
    return ret


N, X = list(map(int, input().split()))
X -= 1  # X枚目まで食べるという扱いにする (0based index)

lng = ret_length(N)
nP = ret_nP(N)

# print(lng, nP, X)

ans = 0
l = 0
r = lng - 1
# while r - l > 0:  # lrが隣り合ったら終わり
# 二分探索で無限にバグらせた
while r - l >= 0:  # lrかぶるまで回す
    m = (r + l) // 2  # 必ず真ん中を指す
    if r == X:
        ans += nP
        break
    elif l == X:
        break

    nP = nP // 2
    if m == X:
        ans += nP + 1
        break
    if X > m:
        # Bの数を足す
        ans += nP + 1
        l = m + 1
        r -= 1
    else:
        l += 1
        r = m - 1

print(ans)
