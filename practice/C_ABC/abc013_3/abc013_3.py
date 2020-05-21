# https://atcoder.jp/contests/abc013/tasks/abc013_3
# 必要な分を最初からm日掛けて食べればよい
# すると出費なしの日は(N-m)日→E(N-m)-H の満腹度を満たす金額が最小になる選び方
# B/A, D/C基本的にはコスパのいい方を使うのが良さそうだが...?

# 満腹度がX以上になる最小の組み合わせは？
# 全探索？
#
# B/Aのほうがコスパがいい場合は貪欲で満腹度を満たすまで食えばいい?
# D/C?


# mを二分探索すればいいじゃない？
# いや、出費を二分探索
# X円で過ごせると仮定→過ごせるギリギリを探す
# 判定→X円以下で必要な満腹度を超すことができるか？
#

# 違う！二分探索に縛られすぎ


# でも最初に食べる、回数が重要っていう考察は合ってた！
# 以下解説AC
# それぞれX回、Y回食事するとする。
# 素直にX,Yを全探索して、満腹度の条件を満たすうちの最小の金額を探せばよい O(N^2)
# 満腹度の条件 H+BX+DY - (N-X-Y)E > 0
# どうやってオーダーを落とす？式変形すれば→ Y > ((N-X)E-H-BX) / (D+E)を満たすYはすべて満腹度の条件を満たすし、これを満たす最小のYを探せば良い！

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


N, H = read_ints()
A, B, C, D, E = read_ints()

ans = 1 << 63
for X in range(N + 1):
    Y = max(((N - X) * E - H - B * X) // (D + E) + 1, 0)
    ans = min(ans, X * A + Y * C)
print(ans)
