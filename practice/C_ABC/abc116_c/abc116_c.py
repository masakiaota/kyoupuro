# https://atcoder.jp/contests/abc116/tasks/abc116_c
N = int(input())
H = list(map(int, input().split()))


def ret_n_miz(ls: list):
    ret = 0  # 水やり回数
    pre = 0  # 水やりが必要なければ0
    for l in ls:
        if pre == 0 and l > 0:
            ret += 1
        pre = l
    return ret


ans = 0
while sum(H) > 0:
    ans += ret_n_miz(H)
    for i in range(N):
        H[i] = max(0, H[i] - 1)

print(ans)
