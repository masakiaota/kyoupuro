
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


print('Yes' if is_prime(53) else 'No')
print('Yes' if is_prime(295927) else 'No')
