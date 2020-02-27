# https://atcoder.jp/contests/abc110/tasks/abc110_c
# 文字のカウント数についてその種類が同じならばYes

S = input()
T = input()

from collections import Counter
s_cnt = Counter(S)
t_cnt = Counter(T)


# if set(s_cnt.values()) == set(t_cnt.values()) and len(s_cnt) == len(t_cnt):
#     # 実は嘘解法 #文字のカウントの種類について、長さが等しく且つセットも等しいけど、内容が異なる場合は存在する。
#     print('Yes')
# else:
#     print('No')

for (k, v), (kk, vv) in zip(s_cnt.most_common(), t_cnt.most_common()):
    if v != vv:
        print('No')
        break
else:
    print('Yes')
