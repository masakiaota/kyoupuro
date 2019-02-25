N, A, B, C = list(map(int, input().split()))
L = [int(input()) for _ in range(N)]


def dfs(cur, a, b, c):
    if cur == N:
        if min(a, b, c) == 0:  # もし使われない竹があったら,最大のコストを返す
            return float('INF')
        # 最終的に出来上がった三本の竹のコストはどうか
        return sum([abs(_ - __) for _, __ in zip([a, b, c], [A, B, C])])
    sub_cost_A = dfs(cur+1, a+L[cur], b, c) + 10
    sub_cost_B = dfs(cur+1, a, b+L[cur], c)+10
    sub_cost_C = dfs(cur+1, a, b, c+L[cur])+10
    sub_cost_X = dfs(cur + 1, a, b, c)  # どのグループにも使わない
    return min(sub_cost_A, sub_cost_B, sub_cost_C, sub_cost_X)


print(dfs(0, 0, 0, 0)-30)  # a,b,cについて0に連結して＋10のコストをそれぞれ1回受けているので−30して帳尻合わせ
