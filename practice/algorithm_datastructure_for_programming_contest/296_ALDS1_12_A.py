# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/12/ALDS1_12_A
# 本よりこっちのほうがぶっちゃけわかりやすい http://www.deqnotes.net/acmicpc/prim/
# 螺旋本ではダイクストラとの違いをあまり解説していないが、違いはダイクストラのところで考えよう
# 記号は本に合わせる。
# 制約条件によりノードはたかだか100なのでオーダーを木にせずにナイーブに実装できる。本ではdを導入することにより計算量を少し削減している

INF = 10**5
# load data
N = int(input())
M = []  # 隣接行列
Color = []  # 訪問状態を記録 0:未訪問, 1:訪問経験あり, 2:訪問済み(用済み) #そうはこいつが2になっているノードはMST
D = []  # 本で言うd #こいつの役割はP299の図を追うとわかりやすい #MSTに隣接しているノードへのコストをノードのところに記録する
P = []  # 本で言うp # MSTにおける親を記録

for _ in range(N):
    Color.append(0)  # ノードはすべて未訪問
    D.append(INF)  # 無限で初期化しておく
    P.append(None)
    M.append([x if x != -1 else INF for x in map(int, input().split())])

# primのアルゴリズムを適応する
# ノード0からMSTを構築していく #まずはノード0をMSTに追加する
D[0] = 0
P[0] = None  # 親が存在しないことをNoneで示す

while True:  # MSTができるまで繰り返す ちなみにループの一周目は0をMSTに追加するところから始まる
    # MSTに隣接するノードへのpathの中で一番小さなコストを見つける、ついでにそのノード番号も
    min_cost = INF
    for i in range(N):
        if Color[i] != 2 and D[i] < min_cost:  # 訪問済みかつ最小値を更新したら
            min_cost = D[i]
            u = i  # uにはコストが一番小さい時のノードが追加される

    if min_cost == INF:
        break  # つながりがもうない

    Color[u] = 2  # つながるのに一番コストの小さい訪問済みに(MSTに追加)する

    for v in range(N):
        if Color[v] != 2 and M[u][v] != INF:  # MSTに追加されていなくてかつ、uとつながっている
            if M[u][v] < D[v]:  # 新たに追加したノードuからvに行くほうがコストが小さいならばそれをメモする
                D[v] = M[u][v]
                P[v] = u  # vの親の候補としてuを代入しておく(だいぶ無駄では？)
                Color[v] = 1  # 一度見たことあるのはgrayにしておくが、ぶっちゃけ意味はない

# これにてPに木構造が入ったのでこれをもとにコストを計算すれば良い
# ぶっちゃけこの操作は上のwhileに組み込み可能なんだけどあえてね
ans = 0
for i, p in enumerate(P[1:], start=1):
    ans += M[i][p]

print(ans)
