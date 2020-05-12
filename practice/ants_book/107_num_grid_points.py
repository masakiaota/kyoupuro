P1 = (1, 11)
P2 = (5, 3)

# dx,dyを同じ数で割ったときどちらも整数ならば、grid point上にある(ベクトルをイメージするとわかりやすい)
# prid pointの数は最小のベクトル(dx/gcd(dx,dy),dy/gcd(dx,dy))の個数-1なので、gcd(dx,dy)-1となる

from math import gcd
print(gcd(P2[0] - P1[0], P2[1] - P2[1]) - 1)
