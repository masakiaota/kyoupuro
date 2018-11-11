N = int(input())
T = []
for i in range(N):
    T.append(int(input()))

T.sort()


def gcd(a, b):
    while (b != 0):
        tmp = a
        a = b
        b = tmp % b

    return a


def lcm(a, b):
    return (a*b)//gcd(a, b)


ans = T[0]
for t in T[1:]:
    ans = lcm(ans, t)
    # print(ans)

print(ans)


# ちなみにgcdは以下のように再帰関数の形で記述できる
# 思いつくか！！！
def gcd(a, b):
    if b == 0:
        return a
    return gcd(b, a % b)
