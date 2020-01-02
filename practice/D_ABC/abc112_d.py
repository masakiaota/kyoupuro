# https://atcoder.jp/contests/abc112/tasks/abc112_d

# 答えansの満たす条件
# ans<=M//Nである
# M%ans==0である


# 仮説:M//Nからansを--して、M%ans==0となるansが答え
# でももしN=2, M=10*9+7とかだったらTLEしそう→しなかった(コーナーケースに助けられた？)
# 2 999999937でTLEしてしまうのでこれは嘘解答

N, M = list(map(int, input().split()))

# 以下嘘解答
# ans = M // N
# for a in range(ans, 0, -1):
#     if M % a == 0:
# print(a)
# exit()

# 本当の解答
ans_temp = M // N
if M % N == 0:
    print(ans_temp)
    exit()


def make_divisors(n, sort=False):
    divisors = []
    for i in range(1, int(n**0.5) + 1):
        if n % i == 0:
            divisors.append(i)
            if i != n // i:
                divisors.append(n // i)
    if sort:
        divisors.sort()
    return divisors


print(max(make_divisors(M), key=lambda x: 0 if x > ans_temp else x))
