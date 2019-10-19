# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/9/ALDS1_9_B
# heapの完全二分木としての表現は二分探索木のときとルールが違いのでこんがらがらないように注意
# 詳しくはP233と238の図
# 勉強にはなるが解くだけならpythonのライブラリでok
from heapq import heapify, heappop, heappush, heappushpop
N = input()
H = [-x for x in map(int, input().split())]

heapify(H)
print('', *[-x for x in H])
