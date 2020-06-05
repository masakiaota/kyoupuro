# なんかチケットの使い方(8!)の全探索でダイクストラ法を使えば行けそうだが...
# ここは解説どおりDAGで解く
# 調べると拡張ダイクストラとかいう方法が出てくる
# やってることは拡張ダイクストラと同じだけど、巡回路がないからダイクストラじゃなくても解けるってことか


from collections import deque, defaultdict
# 入力例
n = 2
m = 4
a = 2 - 1  # 0 based indexに変換
b = 1 - 1
t = [3, 1]

road = {0: [(2, 3), (3, 2)],  # from:[(to, cost),...]
        1: [(2, 3), (3, 5)],
        2: [(0, 3), (1, 3)],
        3: [(0, 2), (1, 5)]}


# verify用 ただしMLEになる
# http://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=1138&lang=jp
# n, m, p, a, b = map(int, input().split())
# a, b = a - 1, b - 1
# t = list(map(int, input().split()))
# road = defaultdict(lambda: [])
# for _ in range(p):
#     x, y, z = map(int, input().split())
#     road[x - 1].append((y - 1, z))
#     road[y - 1].append((x - 1, z))


# 実装
INF = 2 ** 31  # 2147483648 > 10**9
S_max = pow(2, n)
D = [[INF] * S_max for _ in range(m)]  # D[v][S]...乗車券の状態がSでノードvにたどり着ける時間の最小
D[a][S_max - 1] = 0  # 乗車券を使わずにaにいることは可能

# 幅優先探索ながらDを埋めてく
que = deque([(a, S_max - 1, 0)])  # (現在いるノード、乗車券の状態、そこまでの時間)
while que:
    v, S, time = que.popleft()
    for to, cost in road[v]:
        for i in range(n):
            if (S >> i) & 1 == 0:
                continue  # i番目の乗車券は使えないので処理しない
                # 乗車券が使えなくなれば自動的にwhileが止まる
            S_new = S - (1 << i)
            time_new = time + (cost / t[i])
            que.append((to, S_new, time_new))
            D[to][S_new] = min(D[to][S_new], time_new)

# print(*D, sep='\n')

# bにたどり着くための最小コストの取得
ans = INF
for S in range(S_max):
    ans = min(ans, D[b][S])

print(ans if ans != INF else 'Impossible')  # ok
