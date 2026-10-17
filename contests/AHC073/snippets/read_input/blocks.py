# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 入力: N M K、座標 N 行、重み付き辺 M 行、別の座標 K 行。
# 辺の頂点番号は入力が1始まりの例。0始まりの入力なら -1 を外す。
# 戻り値: N, M, K, points, edges, targets。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N, M, K = map(int, input().split())
    points = [tuple(map(int, input().split())) for _ in range(N)]
    edges = []
    for _ in range(M):
        u, v, w = map(int, input().split())
        edges.append((u - 1, v - 1, w))
    targets = [tuple(map(int, input().split())) for _ in range(K)]
    return N, M, K, points, edges, targets
