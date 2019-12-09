# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/all/ALDS1_1_C
# 素数判定する場合は表をつくってしまうのが効率的
# それがエラトステネスの篩
# ただしpythonだとメモリエラーになる

from math import sqrt


def ret_erators(N: int):
    is_prime = [True] * (N + 1)

    # 0と1は素数ではない
    is_prime[0] = False
    is_prime[1] = False

    for i in range(2, int(sqrt(N)) + 1):
        if is_prime[i]:
            j = i * 2  # iの倍数は素数ではない
            while j < N + 1:
                is_prime[j] = False
                j += i
    return is_prime


# load data
N = int(input())
ans = 0
x_ls = []
x_max = 0
for _ in range(N):
    x = int(input())
    x_ls.append(x)
    x_max = max(x_max, x)

is_prime = ret_erators(x_max)
for x in x_ls:
    ans += is_prime[x]
print(ans)
