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


X, Y, Z, K = read_ints()
A = read_ints()
B = read_ints()
C = read_ints()
A.sort(reverse=True)
B.sort(reverse=True)
C.sort(reverse=True)

A = A+[-1000000000]
B = B+[-1000000000]
C = C+[-1000000000]

abcidx = [0, 0, 0]  # A,B,C idx

for _ in range(K):
    ans = A[0]+B[0]+C[0]
    print(ans)

    tmp = max([(A[1], 'a'), (B[1], 'b'), (C[1], 'c')])
    if tmp[1] == 'a':
        ans = ans - A[0] + A[1]
        del A[0]
    elif tmp[1] == 'b':
        ans = ans - B[0] + B[1]
        del B[0]
    elif tmp[1] == 'c':
        ans = ans - C[0] + C[1]
        del C[0]
