a, b = map(int, input().split())


def f(x):
    uru = x // 4
    uru -= x // 100
    uru += x // 400
    return uru


print(f(b) - f(a - 1))
