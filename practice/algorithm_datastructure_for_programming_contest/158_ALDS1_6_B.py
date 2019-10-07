# https: // onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/6/ALDS1_6_B
# クイックソートを理解するために必要な問題。まあやるだけ


def partition(A, p, r):
    x = A[r]
    i = p-1
    for j in range(p, r):
        if A[j] <= x:
            i = i+1
            A[i], A[j] = A[j], A[i]
    A[i+1], A[r] = A[r], A[i+1]
    return i+1


input()
A = list(map(int, input().split()))

idx = partition(A, 0, len(A) - 1)
A[idx] = f'[{A[idx]}]'
print(*A)
