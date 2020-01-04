# https://atcoder.jp/contests/abc105/tasks/abc105_c
# -2進数
# 前に考えたときはわからなかったけど、数式イジイジしたら解けた


N = int(input())

if N == 0:
    print(0)
    exit()

ans = ''
while N != 0:
    if N & 1:  # 奇数ならbitが立つのは共通する性質
        ans += '1'
        N /= -2  # -0.5ずれるのを吸収する処理
        N = int(N + 0.5)

    else:
        ans += '0'
        N //= -2


print(ans[::-1])
