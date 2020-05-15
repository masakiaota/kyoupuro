# ポイントとしてはb-a<10^6 なのでこの区間内での走査は行える
# またこの区間内の倍数は√bまでの素数を列挙すればok


def ret_eratos(N: int):
    '''エラトステネスの篩'''
    is_prime = [True] * (N + 1)
    # 0と1は素数ではない
    is_prime[0] = False
    is_prime[1] = False
    for i in range(2, int(N ** 0.5) + 1):
        if is_prime[i]:
            for j in range(i * 2, N + 1, i):  # iの倍数は素数でない
                is_prime[j] = False
    return is_prime


def range_eratos(a, b):
    '''[a,b)内の素数の配列 is_prime[x-a]で取得すること'''
    root_b = int(b**0.5) + 1
    is_prime_small = [True] * (root_b + 1)  # [0,√b)の篩
    is_prime_small[0] = False  # 0,1は素数でない
    is_prime_small[1] = False

    is_prime = [True] * (b - a)  # [a,b)の篩
    if a == 0:  # コーナーケース用
        is_prime[0] = False
        is_prime[1] = False
    elif a == 1:
        is_prime[0] = False

    for i in range(2, root_b + 1):
        if is_prime_small[i]:
            # smallの更新
            for j in range(i * 2, root_b + 1, i):
                is_prime_small[j] = False
            # is_primeの更新
            s = ((a - 1) // i + 1) * i  # a以上のiの倍数で最小のもの
            for j in range(max(2 * i, s), b, i):  # sが素数の可能性もあるのでmaxを取る
                is_prime[j - a] = False
    return is_prime


print(sum(range_eratos(22, 37)))
print(sum(range_eratos(22801763489, 22801787297)))


def verify(a, b):
    is_prime1 = ret_eratos(b - 1)
    is_prime2 = range_eratos(a, b)
    for i in range(a, b):
        if is_prime1[i] != is_prime2[i - a]:
            print(i, is_prime1[i], is_prime2[i])

    return True


verify(1000, 1000000)  # ok
verify(0, 1000000)  # ok
verify(1, 1000000)  # ok
