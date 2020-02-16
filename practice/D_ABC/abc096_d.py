# https://atcoder.jp/contests/abc096/tasks/abc096_d
# 制約は小さい
# まず 55555以下の素数をエラトステネスの篩であぶり出す

# 55555以下の素数からN個選ぶ。N個の中からどの5つの素数を選んでも合成数となるのが良い

# 性質を列挙
# 5の倍数+1の素数を5a+1とする #aはなんでも
# このとき、5つの和は 5a+1 + 5b+1 + ... + 5d+1 で5で割り切れる合成数となる
# よって5の倍数+1となるような素数を探して列挙すれば良い


def ret_erators(N: int):
    # エラトステネスの篩
    is_prime = [True] * (N + 1)

    # 0と1は素数ではない
    is_prime[0] = False
    is_prime[1] = False

    for i in range(2, int(N**0.5) + 1):
        if is_prime[i]:
            j = i * 2  # iの倍数は素数ではない
            while j < N + 1:
                is_prime[j] = False
                j += i
    return is_prime


is_prime = ret_erators(55555)
# print(len(is_prime))
# print(sum(is_prime))
candi = []
for i in range(55555):
    n = i * 5 + 1
    if n > 55555:
        break
    if is_prime[n]:
        candi.append(n)


print(*candi[:int(input())])
