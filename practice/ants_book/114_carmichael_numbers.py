# まず素数じゃないことを確かめてから
# 素直にxを[2,n)まで変化させて成り立つことを確かれば良い
# オリジナルの問題ではもっと高速化させないとTLEになるっぽい(そもそもpowを使わない)けど今はpowの練習なので


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


def solve(n):
    if is_prime(n):
        print('No')
        return

    for x in range(2, n):
        if pow(x, n, n) != x:  # pythonのpowに繰り返し二乗法は入っている
            print('No')
            return
    print('Yes')


solve(17)
solve(561)
solve(4)
