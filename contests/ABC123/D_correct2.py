# 全探索に終了条件を追加することで計算量を削減させるやり方
# 爆速だった
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


X, Y, Z, K = read_ints()
A = read_ints()
B = read_ints()
C = read_ints()

A.sort(reverse=True)
B.sort(reverse=True)
C.sort(reverse=True)

ABC = []
for a in range(X):
    if a + 1 > K:
        break
    for b in range(Y):
        if (a + 1) * (b + 1) > K:
            break
        for c in range(Z):
            # a,b,cはA,B,Cの上から何番目に大きいか
            if (a + 1) * (b + 1) * (c + 1) > K:
                break
            ABC.append(A[a] + B[b] + C[c])


ABC.sort(reverse=True)
ABC = ABC[:K]
print(*ABC, sep='\n')
