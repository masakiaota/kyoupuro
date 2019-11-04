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

if S[0] == '>':
    S = '<' + S
    # print(S)

if S[-1] == '<':
    # print('this?')
    S = S + '>'


# 連長圧縮する
s_pre = '<'
sum_s_tmp = 0
rencho = []  # 偶数番に<の個数、奇数番に>の個数が入る
for s in S:
    if s_pre == s:
        sum_s_tmp += 1
    else:
        rencho.append(sum_s_tmp)
        sum_s_tmp = 1
        s_pre = s
rencho.append(sum_s_tmp)

# print(rencho)
ans = 0
for a_big, a_small in zip(rencho[0::2], rencho[1::2]):
    if a_big < a_small:
        a_big, a_small = a_small, a_big
    ans += sum(range(a_big + 1))
    ans += sum(range(a_small))


print(ans)
