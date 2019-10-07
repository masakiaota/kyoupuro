# https: // onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/6/ALDS1_6_A
# 要素のmaxであるkさえわかっていればO(n+k)でソートできる高速なソート。天才かと思った。
# ソートしたい配列の数字をidxに対応させて、count 配列を作成. count配列を累積和。それがidxに対応しているというびっくり方法。
# ! 0 based-indexにするために、本とは違う実装

from itertools import accumulate

n = int(input())
A = list(map(int, input().split()))
k = max(A)

C = [0]*(k+1)
B = [-1]*n

# counting
for a in A:
    C[a] += 1

# accumulation
C_acc = list(accumulate(C))

# sorting
# for i in range(n - 1, -1, -1):  # Aの最後の要素から最初の要素までアクセスするために
#     C_acc[A[i]] -= 1  # 0based indexにしたいので先に-1しておく
#     B[C_acc[A[i]]] = A[i]

# こちらのほうが早いかなぁと思ったけど、そんなことなかった。
# まあpythonぽい書き方ということで。
for a in A[::-1]:  # Aの最後の要素から最初の要素までアクセスするために
    C_acc[a] -= 1  # 0based indexにしたいので先に-1しておく
    B[C_acc[a]] = a

print(*B)

# オーダー的には大きいpythonのsortのほうが若干早かった...
