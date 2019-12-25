# https://atcoder.jp/contests/abc137/tasks/abc137_c
# アナグラムの個数を求める問題
# アナグラムの条件→各alphabetの文字数が一致 もしくは ソートすると並び順が同じでもok
# 同じ文字カウント(ソートして同じ文字列)がN個あるとき、i,jの組み合わせは1/2(n*n-n)となる (組み合わせを考えて確かめてみよ)

from collections import Counter

char_sorted = []
N = int(input())
for _ in range(N):
    s = input()
    char_sorted.append(''.join(sorted(s)))

ans = 0
for a in Counter(char_sorted).values():
    ans += ((a - 1) * a) // 2
print(ans)
