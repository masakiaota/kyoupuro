# 良さげだけどバグが取れない！！！！
# とれた...

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
S = read()[:-1]
sum_0 = [0]
# 0のブロックの累積和を作成する
#  1 1 0 1 0 0 1だったら
# 0 0 0 1 1 2 2 2みたいな
now = '1'
for s_next in S:
    if s_next == '0' and now == '1':
        sum_0.append(sum_0[-1] + 1)
    else:
        sum_0.append(sum_0[-1])
    now = s_next

sum_0_2 = [0]
#  1 1 0 1 0 0 1だったら
# 0 0 0 1 1 1 2 2みたいな
# もし0で終わっていたら後ろに追加する。
for s_now, s_next in zip(S[:-1], S[1:]):
    if s_next == '1' and s_now == '0':
        sum_0_2.append(sum_0_2[-1] + 1)
    else:
        sum_0_2.append(sum_0_2[-1])

if S[-1] == '0':
    sum_0_2.append(sum_0_2[-1] + 1)
else:
    sum_0_2.append(sum_0_2[-1])

# print(S)
# print(sum_0)
# print(sum_0_2)

# [left,right)で考える
left = 0
right = 1
ans = 1  # 解初期値

# right-leftが答え
# sum_0[right]-sum0[left]がKを超えたらleftを進める
# print(sum_0)
# しゃくとり法で溶けそうでは？
for left in range(N - 1):
    while right < N:
        nextright = right+1
        # ここにバグの原因がここにあった。半端な00..00の途中から半端な00..00の途中までの引き算は正しく行われない
        if sum_0[nextright] - sum_0_2[left] > K:
            break
        right += 1
    print(right, right < N)
    ans = max(ans, right - left)  # 大きい方に更新
    print(left, right, ans, sum_0[nextright] - sum_0_2[left])

print(ans)
