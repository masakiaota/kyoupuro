# いろいろな解き方が存在すると思うが、個々ではBFSを用いて解くことにする。
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


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
    return [read() for _ in range(H)]


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


N, M = read_ints()

A = read_ints()[1:]
B = read_ints()[1:]
C = read_ints()[1:]

from collections import deque

que = deque([(A.copy(), B.copy(), C.copy(), 0)])

# 幅優先探索
while que:
    A, B, C, cost = que.popleft()
    # print(cost)
    # print(A)
    # print(B)
    # print(C)
    # 条件を満たしたら終了
    if cost > M:
        ans = -1
        break

    if len(A) == N or len(C) == N:
        ans = cost
        break

    # 探索の条件分岐
    if A and B:
        if A[-1] > B[-1]:
            que.append((A[:-1], B + [A[-1]], C, cost + 1))
        if B[-1] > A[-1]:
            que.append((A + [B[-1]], B[:-1], C, cost + 1))
    elif A:
        # Aのだけあった場合
        que.append((A[:-1], B + [A[-1]], C, cost + 1))
    else:
        # Bだけあった場合
        que.append((A + [B[-1]], B[:-1], C, cost + 1))

    if B and C:
        if C[-1] > B[-1]:
            que.append((A, B + [C[-1]], C[:-1], cost + 1))
        if B[-1] > C[-1]:
            que.append((A, B[:-1], C + [B[-1]], cost + 1))
    elif C:
        que.append((A, B + [C[-1]], C[:-1], cost + 1))
    else:
        que.append((A, B[:-1], C + [B[-1]], cost + 1))
    # print(que)
    # visited処理をしたいがどうすればよいのかわかってない
    # →前々回の手を持っておけばもとに戻っているか確かめることはできる。
print(ans)
