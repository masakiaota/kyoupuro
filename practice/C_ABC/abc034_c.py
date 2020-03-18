# https://atcoder.jp/contests/abc034/tasks/abc034_c
# easyか? modとりながらcombination


# mod取りながらcombination
def combination_mod(n, r, mod):
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


W, H = map(int, input().split())
print(combination_mod(W + H - 2, W - 1, 10**9 + 7))
