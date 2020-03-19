# https://atcoder.jp/contests/abc027/tasks/abc027_c

N = int(input()) + 1
depth = -1
n = N
while n:
    n //= 2
    depth += 1

# print(depth)
# 深さが奇数なら高橋くんはなるべく左(1を足さない戦略)がよい
# 青木くんは右
t_add = 0 if depth & 1 else 1
a_add = 1 - t_add

# シミュレーション
n = 1
player = 0  # 0が青木くん 1が高橋くん
while n < N:
    player = 1 - player
    n *= 2
    n += a_add if player == 0 else t_add
print('Aoki' if player == 1 else 'Takahashi')
