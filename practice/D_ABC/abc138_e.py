# https://atcoder.jp/contests/abc138/tasks/abc138_e

# s'は無限回繰り返されているとみなす
# sの中に含まれていない文字でtが構成されていたら速攻で-1

'''
1234567    ...                      33
contest contest contest contest contest contest
     s      e     nte     n     c   e

だけどcontestを無限に生成して、マッチングさせていくのでは最悪10**10になってTLEになってしまう。
貪欲を高速にやる必要あり。
つまりs`を回すんじゃなくて、tを回してできる方法を考える
t=sentenceなら 1文字目から次にsがでてくるのは5文字後、
6文字目から次にeが出てくるのは6文字後、
12文字目から...みたいに計算できる。 (12文字目は 12%7文字目に対応 (=0のときだけ7文字目に対応))

この操作に必要なのはsのi文字目(1-based-idx)において次に文字cが出てくるのは何文字後か、である。
'''

# from collections import defaultdict
# import sys
# read = sys.stdin.readline

# s = read()[:-1]
# t = read()[:-1]
# if set(t) - set(s):
#     print(-1)
#     exit()

# # 前処理パート
# S = s + s
# # 逆向きから走査することでi文字目に置いて次にiが出てくるのはどこか見つける
# tmp = {}  # いま文字keyが最後に出たのはいくつあとか
# skiptable = [None] * (len(s) + 1)

# # 前処理の前処理、
# for i in range(len(S) - 1, -1, -1):
#     moji = S[i]
#     for key in tmp.keys():
#         tmp[key] += 1  # すべて一文字あとになる
#     if i < len(s):
#         skiptable[i + 1] = tmp.copy()  # ミュータブルかつアドレス参照に注意
#     tmp[moji] = 0  # その文字は0文字あとに修正


# # 本処理
# # 最初にうまく飛べる点を探索
# for i, ss in enumerate(s):
#     if ss == t[0]:
#         now = i + 1
#         break

# for tt in t[1:]:
#     idx = now % len(s)
#     idx = idx if idx else len(s)
#     now += skiptable[idx][tt]
# print(now)

'''
きれいに再実装
'''

from bisect import bisect_right
from collections import defaultdict
import sys
read = sys.stdin.readline

s = read()[:-1]
t = read()[:-1]
# 即時終了
if set(t) - set(s):
    print(-1)
    exit()

# 前処理
# 0文字目から見たときにその文字が何文字目にあるかを記録しておく
mojiidx = defaultdict(lambda: [])
for i, ss in enumerate(s + s):
    mojiidx[ss].append(i)

# # i文字目において、次に文字cが出てくるのは何文字あと？
# skiptable = []
# for i in range(0, len(s)):
#     tmp = {}
#     for c, ls in mojiidx.items():
#         tmp[c] = ls[bisect_right(ls, i)] - i #ここのオーダでかくね？
#     skiptable.append(tmp)
# これは後の処理に吸収できる


# 本処理
# 最初にうまく飛べる点を探索
for i, ss in enumerate(s):
    if ss == t[0]:
        now = i
        break

for tt in t[1:]:
    idx = now % len(s)
    # 前処理の列から文字ttが何文字あとに出現するかを探索する
    ls = mojiidx[tt]
    tmp = ls[bisect_right(ls, idx)] - idx
    now += tmp
print(now + 1)
