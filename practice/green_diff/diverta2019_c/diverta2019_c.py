# https://atcoder.jp/contests/diverta2019/tasks/diverta2019_c

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# greedyっぽいなぁ
# ABとなる部分を最大化したい
# *B → A*はむだ、*A→B*となる文字列はそのように連結させたほうがよい
# つまり新たにAB文字列になるのはmin(*Aの数,B*の数)
# ただしB*Aを満たす文字列に関してはどちらにもなりうるので、好きにpaddingできる

N = int(input())
cnt = end_A = start_B = both = 0
for _ in ra(N):
    s = input()
    cnt += s.count('AB')

    if s.endswith('A') and s.startswith('B'):
        both += 1
    elif s.endswith('A'):
        end_A += 1
    elif s.startswith('B'):
        start_B += 1


# 処理の順番も大事
# bothから考えると、bothは基本的にboth−1個のABをつくり,B*Aの文字列に化ける
# つぎにend_Aやstart_Bについて考える。これらによってmin(end_A,start_B)個できる。しかし、B*Aが存在する場合はこれを途中に挟まなければ最大値にならない。
# つまり以下のように処理すれば良い。
# both=0の場合→min(end_A,start_B)
# both>0の場合→ end_A=endB=0→both-1, end_A or endB=0→both, それ以外→both+min(end_A,start_B)
# ∵まずmin(end_A,start_B)個の組をつくりそのうちの一つをばらしてB*Aを挿入するのが最適、min()-1(ばらした分)+2(増える分)+both-1(B*Aの中の'AB'の数)だから


add = min(start_B, end_A)
if both > 0:
    if end_A == 0 and start_B == 0:
        add += both - 1
    else:
        add += both

# print(cnt, add)
print(cnt + add)
