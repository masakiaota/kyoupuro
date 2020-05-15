# 素数というか整数論関係


def is_prime(x: int):
    # 高速素数判定
    if x == 1:
        return False
    if x % 2 == 0:  # 定数倍高速化
        return x == 2

    for i in range(3, int(x**0.5) + 1, 2):
        if x % i == 0:
            return False

    return True


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
