# https://atcoder.jp/contests/abc112/tasks/abc112_c


# ピラミッドの中心は必ず0-100の中にある
# Hは1以上
# 与えられる情報だけで一意に定まる

# 中心座標をa,bと置いたとき、ある点x,yにおける高さは
# h(x,y; a,b)=max(H-|x-a|-|y-b|,0)である。
# ここで、真の高さとの誤差としてe(x,y;a,b)=|h(x,y;a,b) - h(x,y;Cx,Cy)|とすると。
# 任意のx,yでe=0となるa,bが答え。しかしHはわからないので0とする。
# そうすると任意のx,yでe(x,y;a,b)=constとなるa,bが答え。(ただしmaxの扱いが面倒なのでh==0は無視して処理を行う)
# またその定数constはHである

# 座標は10**4点。Nはたかだか100なので 最悪10**6回計算。間に合う。
# あー結局全探索してるんだから問題の定義を借りて素直に実装すればバグらせずに済んだな

import numpy as np
from itertools import product

N = int(input())
X, Y, H = [], [], []
for n in range(N):
    x, y, h = list(map(int, input().split()))
    if h == 0:
        continue
    X.append(x)
    Y.append(y)
    H.append(h)

if len(H) == 1:
    print(X[-1], Y[-1], H[-1])
    exit()

X = np.array(X)
Y = np.array(Y)
H = np.array(H)


def is_ok(X, Y, H, a, b):
    height = H + np.abs(X - a) + np.abs(Y - b)
    if (height == height[-1]).all():
        return True, height[-1]
    else:
        return False, False


for a, b in product(range(101), range(101)):
    flg, ansH = is_ok(X, Y, H, a, b)
    if flg:
        ans = (a, b, ansH)

print(*ans)
