# https://atcoder.jp/contests/abc029/submissions/10940214
# dfs全探索
N = int(input())


def dfs(now, i):
    # j....いままでで最大の文字のidx
    # 文字を作って返す
    if i == N - 1:
        print(now)
        return
        # return now
    for nx in ('a', 'b', 'c'):
        dfs(now + nx, i + 1)


dfs('', -1)
