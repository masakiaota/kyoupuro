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


S = input()
K = int(input())

pre = None
if K < 10:
    first_cnt = 0
    for s in S*K:
        if s == pre:
            first_cnt += 1
            pre = None
        else:
            pre = s
    print(first_cnt)
    exit()

pre = S[0]
first_cnt = 0

for s in S[1:]:
    if s == pre:
        first_cnt += 1
        pre = None
    else:
        pre = s

continuous_cnt = 0
for s in S:
    if s == pre:
        continuous_cnt += 1
        pre = None
    else:
        pre = s

continuous_cnt2 = 0
for s in S:
    if s == pre:
        continuous_cnt2 += 1
        pre = None
    else:
        pre = s

if continuous_cnt == continuous_cnt2:
    print(first_cnt+continuous_cnt*(K-1))
elif K % 2 == 1:
    print(first_cnt+(continuous_cnt+continuous_cnt2)*((K-1)//2))
else:
    print(first_cnt+continuous_cnt*((K-1)//2+1)+continuous_cnt2*((K-1)//2))
    # print(first_cnt, ((K-1)//2))
