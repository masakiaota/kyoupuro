# https://atcoder.jp/contests/abc142/tasks/abc142_c
# 最小コストソートの際に使った、ソートしたあとの要素についてソート前の並び順が知りたい

#  AとA_sortedでA_sorted[j]はA[i]の要素であるiの配列が知りたい

# pythonっぽい書き方
N = int(input())
A = list(map(int, input().split()))
tmp = sorted([(a, n) for n, a in enumerate(A, start=1)])
print(*[x[1] for x in tmp])


# まつり縫いみたいなやり方
# aの要素でアクセスできる形式で、何番目の要素だったのかを保持しておけば良い #実はこっちのほうが処理が早かったりする(タプルのソートに時間がかかる？)
tmp = [None] * (N + 1)
for i, a in enumerate(A, start=1):
    tmp[a] = i
A.sort()
print(*[tmp[a] for a in A])
