# https://atcoder.jp/contests/code-festival-2017-quala/tasks/code_festival_2017_quala_c
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


# 回文の二次元版
# 1次元回文→文字数が奇数だったら,一つの文字以外出現回数はすべて2の倍数。偶数だったら出現回数はすべて2の倍数
# 二次元では？
# H,Wがともに偶数なら→すべての文字の出現回数は4の倍数
# Hが奇数、Wが偶数なら→4の倍数でない要素を、真ん中に詰めていきたい(そのような要素がWについても回文になっているか評価)
# つまり4の倍数が(W*(H-1))個分ある かつ それ以外の要素は2の倍数ある
# Wが奇数、Hが偶数→↑WHをひっくり返すだけ
# W,Hがともに奇数→(W-1)*(H-1)個分ある かつ それ以外の要素は1つ以外2の倍数である

# 上記を一般化すると、二次元回文が満たすべき条件は、
# - 同一文字4個セットは(W-(W&1))*(H-(H&1))//4 個以上ある
# - 同一文字奇数個の要素はちょうどH&1&W個ある(両方奇数のとき1になる)
# - その他の要素はすべて2の倍数個である(ただしこの条件は2番めの条件に内包される)

from collections import Counter

H, W = read_ints()
cnt = Counter()
for _ in range(H):
    cnt.update(input())

# コーナーケース処理
# if H == 1 or W == 1:


num4 = 0
num1 = 0
for v in cnt.values():
    if v & 1:  # 条件2
        num1 += 1

    while v >= 4:  # 条件1の4文字セットを作る 4以上なら4個セットを作ることができる
        v -= 4
        num4 += 1

if num4 >= (W - (W & 1)) * (H - (H & 1)) // 4 and num1 == H & 1 & W:
    print('Yes')
else:
    print('No')
