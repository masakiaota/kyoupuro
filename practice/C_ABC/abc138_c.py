# https://atcoder.jp/contests/abc138/tasks/abc138_c
# その時点で一番価値の低い2つを消費して新たに錬成するのが一番良さそう
# 価値が高いものを何回も割ってしまうと価値がどんどん下がるので。

N = int(input())
V = list(map(int, input().split()))
V.sort(reverse=True)
while len(V) > 1:
    new = (V[-1] + V[-2]) / 2
    V = V[:-2] + [new]

print(V[0])
