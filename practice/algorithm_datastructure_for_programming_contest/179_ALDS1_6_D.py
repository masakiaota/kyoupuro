# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/6/ALDS1_6_D
# 本の解説がわかりやすい
# ここでは本には書いてないことの補足を意識しながらコメントを付けていく
WMAX = 10**4 + 1
n = int(input())
A = list(map(int, input().split()))
B = sorted(A)
is_visited = [False] * n  # 巡回管理用
next_idxs = {}  # Aの要素について、これのkeyを見れば、その数字がsort後にどの位置にあるのかわかるように対応させる
for i, b in enumerate(B):
    next_idxs[b] = i

m_A = min(A)  # globalで一番小さい要素

# ソート後の並び順から巡回可能なグループから答えを計算する
# グループ内の最小の要素をm, グループの和をS, グループ内の要素数をn2とする。
# 具体的には,m*(n2-2) + S もしくは m + S + (1+n2)*m_Aのうち小さい方
ans = 0
for i in range(n):
    if is_visited[i]:
        continue  # 巡回済みならば次へ
    cur = i  # 巡回制御用のidx #探索済みでないiが入るはず
    S = 0  # 合計管理用
    n2 = 0
    m = WMAX  # グループ内の最小値探索用
    while True:
        # まずはもろもろの値を更新
        is_visited[cur] = True
        S += A[cur]
        m = min(m, A[cur])
        n2 += 1
        # ここで次のidxを取得する
        cur = next_idxs[A[cur]]
        if is_visited[cur]:
            break  # もし一周したらおわり
    # ループ内で完結したほうがいいのか外から要素を変えいてきたほうがいいのか
    # 小さい方を採用して答えに足し込む
    ans += min(m*(n2-2)+S, m+S+(1+n2)*m_A)

print(ans)
