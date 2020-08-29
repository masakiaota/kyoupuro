def ret_eratos(N: int):
    '''エラトステネスの篩'''
    is_prime = [True] * (N + 1)
    is_prime[0] = False  # 0と1は素数ではない
    is_prime[1] = False
    for i in range(2, int(N ** 0.5) + 1):
        if is_prime[i]:
            for j in range(i * 2, N + 1, i):  # iの倍数は素数でない
                is_prime[j] = False
    return is_prime


def _make_prime_numbers(N: int):
    # N以下の素数を列挙 -> set
    is_prime = ret_eratos(N)

    primes = set()
    for i, flg in enumerate(is_prime):
        if flg:
            primes.add(i)

    return primes


print(len(_make_prime_numbers(10**8)))
