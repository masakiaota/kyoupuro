# http://poj.org/problem?id=3169
# 蟻本の説明がわかりにくすぎる


def bellman_ford(edges, s, N):
    '''
    edges ... (cost,from,to)を各要素に持つリスト
    s...始点ノード
    N...頂点数

    return
    ----------
    D ... 各点までの最短距離
    P ... 最短経路木における親
    '''
    P = [None] * N
    inf = float('inf')
    D = [inf] * N
    D[s] = 0
    for n in range(N):  # N-1回で十分だけど、N回目にもアップデートがあったらnegative loopを検出できる
        update = False  # 早期終了用
        for c, ot, to in edges:
            if D[ot] != inf and D[to] > D[ot] + c:
                update = True
                D[to] = D[ot] + c
                P[to] = ot
        if not update:
            break  # 早期終了
        if n == len(edges) - 1:
            print(-1)  # 負の閉路が存在するということはそのように並ぶことはできないということ
            exit()
            raise ValueError('NegativeCycleError')
    return D, P

# i<jにおいてi→jは正の重力、j←iは負の重力が働いてるとして、紐をピンと伸ばしたときの最短距離
# と問題を言い換えれば、蟻本P105の図も納得


N = 4
ML = 2
MD = 1
AL = [1, 2]
BL = [3, 4]
DL = [10, 20]
AD = [2]
BD = [3]
DD = [3]


edges = []
for a, b, d in zip(AL, BL, DL):
    edges.append((d, a - 1, b - 1))
for a, b, d in zip(AD, BD, DD):
    edges.append((-d, b - 1, a - 1))

# 未接続のi,i+1に対して順番がひっくり返らないようにする(つまりd[i]+0<=d[i+1]は0離れたい)
for i in range(N - 1):
    edges.append((-0, i + 1, i))


D, P = bellman_ford(edges, 0, N)
print(D)
print(P)

if D[N - 1] == float('inf'):
    print(-2)
else:
    print(D[N - 1])
