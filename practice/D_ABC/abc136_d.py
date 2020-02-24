# https://atcoder.jp/contests/abc136/tasks/abc136_d
# ポケモンの動く床みたいに強制的にある座標に移動すると考える。
# ただし10**100回動いた後に、左右どちら側のマスにいるのかに気をつけなければいけない

# 例
# RRLRL
#   | |ココらへんに3人,2人集まるけど
# 01211 と答えはなる

# RRLについて考えると
# 1  ここの人は無限偶数回移動すると
#   1ここに移動する
#  1 ここの人は無限偶数回移動すると変わらない
#
# 一般化すると、RRRRRRにおいて、一番左のRのidx(r)に対してr-idxが偶数ならばrに移動、奇数ならばr+1に移動
# 更に一般化すると、今までRがn回続いたときr+1にはn//2回、rにはn-n//2回追加される

# Lについても一番右のidx(l)に対してidx-lが偶数ならばlに移動、奇数ならばl−1に移動。
# 更に一般化するとこれからLがn回続くとき、l-1にはn//2人、lにはn-n//2人追加

# 前から回すって考えると Rについては更に一般化した方法、Lについては偶奇を考慮した方法がよろしいかと


# import sys
# read = sys.stdin.readline

# S = read()[:-1]
# ans = [0] * len(S)

# pre = 'R'
# cnt = 1
# r = 0
# for i, s in enumerate(S[1:], start=1):
#     if pre == 'R' and s == 'L':
#         # lの処理の前にRの処理
#         r = i - 1
#         ans[r + 1] += cnt // 2
#         ans[r] += cnt - cnt // 2
#         cnt = 0

#         # l開始の処理
#         l = i
#         pre = 'L'
#         ans[l] += 1
#     elif pre == 'L' and s == 'L':
#         if (i - l) & 1:  # 奇数なら
#             ans[l - 1] += 1
#         else:
#             ans[l] += 1
#     if pre == 'L' and s == 'R':
#         pre = 'R'
#     if pre == 'R' and s == 'R':
#         cnt += 1

#     # print(*ans)

# print(*ans)

# もっと楽に実装できないかなぁ
'''
# ランレングスで実装できない？
'''


def run_length_encoding(s):
    '''
    連長圧縮を行う
    s ... iterable object e.g. list, str 
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx
    '''
    s_composed = []
    s_sum = []
    s_idx = [0]
    pre = s[0]
    cnt = 1
    for i, ss in enumerate(s[1:], start=1):
        if pre == ss:
            cnt += 1
        else:
            s_sum.append(cnt)
            s_composed.append(pre)
            s_idx.append(i)
            cnt = 1
            pre = ss
    s_sum.append(cnt)
    s_composed.append(pre)
    # assert len(s_sum) == len(s_composed)
    return s_composed, s_sum, s_idx


import sys
read = sys.stdin.readline

S = read()[:-1]
ans = [0] * len(S)

S_comp, S_num, S_idx = run_length_encoding(S)
for i, (moji, num, idx) in enumerate(zip(S_comp, S_num, S_idx)):
    if moji == 'R':
        r = S_idx[i + 1]
        ans[r] += num // 2  # 奇数離れてるやつのカウント
        ans[r - 1] += num - num // 2  # 偶数離れてるやつのカウント
    elif moji == 'L':
        l = idx
        ans[l] += num - num // 2  # 偶数離れてるやつのカウント
        ans[l - 1] += num // 2


print(*ans)
