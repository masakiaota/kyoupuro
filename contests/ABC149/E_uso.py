# 入力が10**5とかになったときに100ms程度早い
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


from math import sqrt
N, M = read_ints()

# solve P (P**2 <=Mを満たす最大のP)
P = int(sqrt(M) + 1)  # ひとつ余分にとる (あとで引くため)


A = read_ints()
A.sort(reverse=True)

A = A[:P]
print(A)
tmp = sum(A) * 2 * P  # そこまでの全要素使った場合

sen_cnt = 0
cnt_sum = 0
n_left = P**2 - M
while cnt_sum <= n_left:
    sen_cnt += 1
    cnt_sum += sen_cnt
sen_cnt -= 1

for cnt, youso in zip(range(sen_cnt, -1, -1), A[::-1]):
    print(tmp)
    tmp -= youso * cnt * 2

print(tmp)
