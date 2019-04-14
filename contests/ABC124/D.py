# 良さげだけどバグが取れない！！！！


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


N, K = read_ints()
S = '1' + read()
sum_0 = [0]

for s_new, s_next in zip(S[:-2], S[1:-1]):
    # print(s_new, s_next)
    if s_new == '1' and s_next == '0':
        sum_0.append(sum_0[-1] + 1)
    else:
        sum_0.append(sum_0[-1])

S = S[1:-1]
print(sum_0)

left = 0  # 左端初期値
ans = 1  # 解初期値
right = 0

# right-leftが答え
# sum_0[right]-sum0[left]がKを超えたらleftを進める
# print(sum_0)
# しゃくとり法で溶けそうでは？
for left in range(N-1):
    while (True):
        if right == N-1:
            break
        right += 1
        if sum_0[right + 1] - sum_0[left] > K:
            right -= 1
            break

    # print(left, right)
    ans = max(ans, right - left+1)  # 大きい方に更新
    # print(ans)

print(ans)
