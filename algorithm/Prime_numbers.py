def is_prime(x: int):
    # 高速素数判定
    if x == 1:
        return False
    if x % 2 == 0:
        if x == 2:
            return True
        else:
            return False

    for i in range(3, int(x**0.5) + 1):
        if x % i == 0:
            return False

    return True


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


def factorization(n: int):
    # 素因数分解
    arr = []
    temp = n
    for i in range(2, int(n**0.5) + 1):  # ここにバグがないか心配
        if temp % i == 0:
            cnt = 0
            while temp % i == 0:
                cnt += 1
                temp //= i
            arr.append((i, cnt))

    if temp != 1:
        arr.append((temp, 1))

    if arr == []:
        arr.append((n, 1))

    return arr


def make_divisors(n: int, sort=False):
    # 約数列挙
    divisors = []
    for i in range(1, int(n**0.5) + 1):
        if n % i == 0:
            divisors.append(i)
            if i != n // i:
                divisors.append(n // i)
    if sort:
        divisors.sort()
    return divisors
