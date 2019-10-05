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
    return [read()[:-1] for _ in range(H)]


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


N = read_a_int()
adj_mat = read_map(N)

tmp = []  # 隣接何個？0-3 (条件を満たさないなら0)
# cnt = 0
for i, line in enumerate(adj_mat):
    tmp_cnt = 0
    for j in range(N):
        if line[j] == '1':
            if abs(j - i) != 1:
                tmp_cnt = 0
                break
            elif j == i-1:
                if tmp and tmp[-1] != 0:  # すでに含まれるなら含めない
                    pass
                else:
                    tmp_cnt += 1
            elif j == i+1:
                tmp_cnt += 1

    tmp.append(tmp_cnt)
print(sum(tmp)*2)

# 問題勘違してますねこれは
