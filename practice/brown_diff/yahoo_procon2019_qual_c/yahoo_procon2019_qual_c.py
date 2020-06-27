# https://atcoder.jp/contests/yahoo-procon2019-qual/tasks/yahoo_procon2019_qual_c
# ビスケットを1枚増やす:1回
# ビスケットをA→B枚にする:2回

# 最適戦略
# 手持ち<AのときAまで増やさないと交換できない
# B-A>2のときに限って、交換し続けるのが得にな


import sys


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


K, A, B = map(int, input().split())
if A - 1 >= K:  # コーナーケース、A枚になる前にK回に達する
    exit(K + 1)

if B - A > 2:
    # A枚まで増やす
    K -= (A - 1)
    # 可能な限りA→Bに変換する
    times, r = divmod(K, 2)
    print(r + times * (B - A) + A)
else:
    print(K + 1)
