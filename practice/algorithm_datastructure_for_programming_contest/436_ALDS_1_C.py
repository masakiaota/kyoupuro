# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/all/ALDS1_1_C
# ここに来てめちゃくちゃ簡単になったぞどうした？


from math import sqrt


def is_prime(x):
    if x == 1:
        return False
    if x % 2 == 0:
        if x == 2:
            return True
        else:
            return False

    for i in range(3, int(sqrt(x)) + 1):
        if x % i == 0:
            return False

    return True


# load data
N = int(input())
ans = 0
for _ in range(N):
    x = int(input())
    ans += is_prime(x)

print(ans)
