# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/12/ALDS1_12_A
# 本よりこっちのほうがぶっちゃけわかりやすい http://www.deqnotes.net/acmicpc/prim/
# AtCoderではScipyが使えるので、自分で実装する必要はない https://note.nkmk.me/python-scipy-minimum-spanning-tree/
from scipy.sparse.csgraph import minimum_spanning_tree
# これは高速なクラスカル法を使っている(プリムのアルゴリズムでも工夫すれば同じオーダーになる)

INF = 10**5
# load data
N = int(input())
M = []  # 隣接行列

for _ in range(N):
    M.append([x if x != -1 else INF for x in map(int, input().split())])

# 隣接行列を入力すればオーケー
# MSTの隣接行列を疎行列として返してくる
# <class 'scipy.sparse.csr.csr_matrix'>
'''
  (0, 1)        2.0
  (0, 3)        1.0
  (3, 2)        1.0
  (4, 2)        1.0
'''

print(int(minimum_spanning_tree(M).sum()))
