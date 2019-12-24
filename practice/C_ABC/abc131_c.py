# https://atcoder.jp/contests/abc131/tasks/abc131_c
# A~Bと言われたら、0~B と 0~Aの個数で成り立つのではないか？と考える典型問題

from fractions import gcd


def lcm(a, b):
    g = gcd(a, b)
    return a * b // g


def f(n, c, d):
    # nを含んでcでもdでも割り切れる数の個数を返す
    return n // c + n // d - n // lcm(c, d)


A, B, C, D = list(map(int, input().split()))

print((B - A + 1) - (f(B, C, D) - f(A - 1, C, D)))
