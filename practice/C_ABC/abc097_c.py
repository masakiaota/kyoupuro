# https://atcoder.jp/contests/abc097/tasks/arc097_a
# まずは愚直に実装してみる

# S = input()
# K = int(input())
# candi = []
# for s in range(len(S)):
#     for e in range(s + 1, len(S) + 1):
#         candi.append(S[s:e])
# candi = list(set(candi))
# candi.sort()
# print(candi)
# print(candi[K - 1])


# 辞書順に置いては文字の先頭が重要。
# 一文字目が速いやつから注目していく。
# 一文字目が同じだったらさらにその次の文字を注目する。

# atcoderandatcodeer
# a      a  a       # 1種
# at     an at      # 1種 #tは以下のすべてよりも辞書順があと
#        and        # 1種
#        anda       # 1種
#        andat      #

S = input()
K = int(input())

# 一番速い文字を検索してからそこからK文字見るのが一番早そう。
# ただし、K文字ない場合は辻褄合わせが必要
# てか計算量的に、こんなことしなくてもK文字目まで全列挙すればよくね？

# mi = min(S)
candi = []
for s in range(len(S)):
    for add in range(1, K + 1):
        candi.append(S[s:s + add])

candi = list(set(candi))
candi.sort()
print(candi[K - 1])
