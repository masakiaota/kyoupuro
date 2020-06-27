# https://atcoder.jp/contests/cf17-final/tasks/cf17_final_b

S = input()
# 2文字以上の回分になってしまう条件
# 同じ文字が隣り合う
# 一文字挟んで同じ文字が隣り合う

# 部分文字列が2文字以上の回分にならない文字列
# abcabcabcとかbcabcaとか ようはサイクリックに繰り返せば良い

# 3回繰り返し＋a, 3回繰り返し＋ab,3回繰り返し の3パターンだけあるのだから
# Sのcntがn,n+1,n+2の関係ならば'Yes'

from collections import Counter
cnt = Counter(S + 'abc')
ns = sorted(cnt.values())
n = ns[0]
print('YES' if n <= ns[1] <= n + 1 and n <= ns[2] <= n + 1 else 'NO')
