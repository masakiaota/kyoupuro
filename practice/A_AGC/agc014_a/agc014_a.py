# 適当に10**5ぐらい回して無限に続けられたら-1でよくね？
A, B, C = map(int, input().split())


def update(A, B, C):
    return B // 2 + C // 2, C // 2 + A // 2, A // 2 + B // 2


for i in range(10 ** 5):
    if A & 1 or B & 1 or C & 1:
        print(i)
        exit()
    A, B, C = update(A, B, C)

print(-1)
