import numpy as np
x, y, r = map(int, input().split())
xx1, yy1, xx2, yy2 = map(int, input().split())
# どちらもNOになることはない
# 制約がせいぜい200*200だから全点に対してしらべてもまあ間に合う
# □ < ○ であることは全点について調べるとすぐわかる(これも端っこ4点調べればおk！！！)
# ○ < □ であることは半径をつかった座標4点でわかる

x1 = xx1 - x
x2 = xx2 - x
y1 = yy1 - y
y2 = yy2 - y

sqrt_Ps = np.array([complex(x1, y1),
                    complex(x1, y2),
                    complex(x2, y1),
                    complex(x2, y2)])

if r <= x2 and -r >= x1 and r <= y2 and -r >= y1:  # ○ < □
    # 青のみ
    print('NO')
    print('YES')
elif np.all(np.abs(sqrt_Ps) <= r):
    # elif abs(p1) <= r and abs(p2) <= r and abs(p3) <= r and abs(p4) <= r:
    print('YES')
    print('NO')
else:
    print('YES')
    print('YES')
