# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/5/ALDS1_5_A
# 典型的なナップサック問題。動的計画法を使いたくなるが制約条件がゆるいので全探索でも十分間に合う。
# bit全探索を実装したくなるがここでは練習のために再帰関数で解く。

n = int(input())
A = list(map(int, input().split()))
q = int(input())
M = list(map(int, input().split()))

# i番目を以降の要素でmを作れるかどうかの関数solve(i,m)を再帰的に実装
# 例えば1 5 7 10 21が与えられたときsolve(4,21),solve(4,0)はTrue,solve(4,3)はFalseとすぐわかる
# ではsolve(3, x)はどうだろう。solve(3,31)はsolve(4,31)(つまり3番目の要素を選択しない)がTrueならTrueとなる。
# もしくはsolve(4,21)がTrueならsolve(3,31)もTrue (3番目の要素を選択したら実行可能).
# ここでは更に何も取らない選択も用意しておくと便利だろう。つまりsolve(5,0)ならばTrue
# つまり一般化すると、solve(i,m)はsolve(i+1,m)かsolve(i+1,m-A[i])がTrueならTrueでそれ以外はFalse
# またsolve(hoge,0)は必ず実行可能（再帰の終了条件）
# 以上を用いてDPテーブルを作成できちゃうが今回はあえて再帰的に実装する


from functools import lru_cache
# pythonだとTLEしてしまうためメモ化します


@lru_cache(maxsize=2**12)
def solve(i, m):
    # 終了条件 #dpテーブルの初期条件に相当
    if m == 0:
        return True
        # 数字をAでやりくりした結果切りよく0になるということは、その数字を実現可能だということ
        # 最大でi=len(A)、このときmは必ず0となるので無限ループは回避できる
    if i == n:
        # i==len(A)のときに前のifでm==0となればいいのだが、そうでないならば数字をAやりくりしても実現不可なのでFalseを返す
        return False

    return solve(i+1, m) or solve(i+1, m-A[i])


for m in M:
    if solve(0, m):
        print('yes')
    else:
        print('no')
