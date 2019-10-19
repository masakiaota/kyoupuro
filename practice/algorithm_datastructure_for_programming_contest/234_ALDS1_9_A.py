# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/9/ALDS1_9_A
# 与えられる配列はすでに完全二分木を表す二分ヒープ
# 配列を1オリジンとし、ノードのidxをiとしたとき、そのノードの親はi//2、2*i, 2*i+1。
# これを実装するだけ。(この関係性はP232の図10.2を見れば自明)

N = int(input())
heap = list(map(int, input().split()))

for i, a in enumerate(heap, start=1):  # 1オリジンにするためのstart=1
    print(f'node {i}: key = {a}, ', end='')
    # 親存在するときだけ出力する。
    if i//2 != 0:
        # heapは実際には0オリジンなので-1して調節する
        print(f'parent key = {heap[i//2 -1 ]}, ', end='')
    # 左子が存在するときだけ出力する
    if 2 * i <= N:
        print(f'left key = {heap[2*i - 1]}, ', end='')
    # 右子
    if 2 * i + 1 <= N:
        print(f'right key = {heap[2*i]}, ', end='')
    print()
