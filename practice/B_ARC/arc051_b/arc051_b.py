# https://atcoder.jp/contests/arc051/tasks/arc051_b
# K回再帰するa,bの組み合わせを見つけろ

K = int(input())
# g,0に対してgcdを逆順に行えば良い

q = 0
a, b = 1, 0
r = 1  # 2以上だと制約からオーバーする
for k in range(K + 1):  # 互除法の回数処理を行って
    q_new = b
    b_new = a
    a_new = r * b_new + q_new
    a, b, q = a_new, b_new, q_new


def assertion(a, b):
    cnt = 0

    def gcd(a, b):
        nonlocal cnt
        if b == 0:
            return
        cnt += 1
        return gcd(b, a % b)
    gcd(a, b)
    return cnt


assert assertion(a, b) == K, (assertion(a, b), a, b)
print(a, b)
