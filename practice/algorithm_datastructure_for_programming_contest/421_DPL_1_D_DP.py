# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_D
# P422の解説の方がDPっぽい考え方
# でもお気持ちが書かれていないので、それを図示した
# でもこの問題ではTLE

# load data
N = int(input())
A = [-1]
for _ in range(N):
    A.append(int(input()))

 # LIS (N+1,) ... dpテーブルに相当。A[i]を最後に選んだ最長増加部分列の長さ
 # P (N+1) ...LIS更新を逆順にたどるためのもの、P[i]には、その前のindexが格納されている

# DPテーブルの作成 and 初期状態
LIS = [0] * (N + 1)
P = [-1] * (N + 1)

# 最長増加部分列を求める
for i in range(1, N + 1):
    k = 0  # これがA[i]を後ろにつけるときにどのidxの要素よりもつけたらいいのかを格納するためのidx
    for j in range(0, i):
        if A[j] < A[i] and LIS[k] < LIS[j]:
            # 候補となるのはA[i]よりも小さい要素のidxで かつ
            # 一番Pが大きいidx
            k = j
    LIS[i] = LIS[k] + 1  # LISの更新
    P[i] = k  # LISにおけるA[i]の一つ前の要素がA[k]というのを保存しておくためのもの

print(max(LIS))
