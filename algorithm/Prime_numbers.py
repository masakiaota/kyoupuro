# 素数というか整数論関係


def is_prime(x: int):
    # 高速素数判定
    if x == 1:
        return False
    if x % 2 == 0:
        if x == 2:
            return True
        else:
            return False

    for i in range(3, int(x**0.5) + 1, 2):
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
    if n == 1:
        return []  # 1は素数ではない
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


# def extgcd(a, b):
#     '''ax + by = gcd(a,b) を満たすgcd(a,b),x,yを返す'''
#     if b == 0:
#         return a, 1, 0
#     g, x, y = extgcd(b, a % b)
#     return g, y, x - a // b * y


def extgcd(a, b):  # 非再帰
    '''ax + by = gcd(a,b) を満たすgcd(a,b),x,yを返す'''
    x0, y0, x1, y1 = 1, 0, 0, 1
    while b != 0:  # 互除法の回数処理を行って
        q, a, b = a // b, b, a % b  # 上に伝播させていく
        x0, x1 = x1, x0 - q * x1
        y0, y1 = y1, y0 - q * y1
    return a, x0, y0
