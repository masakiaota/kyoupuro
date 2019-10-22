# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/11/ALDS1_11_A
# やるだけ
# 有向成分を読み込むたびにノード番号に対応する表に書き込んでいけばよい

N = int(input())
for _ in range(N):
    out_ls = [0]*N
    tmp = list(map(int, input().split()))
    # id = tmp[0]-1 #idは順番通り与えられるので実はこれもいらない
    if tmp[1] != 0:
        for adj in tmp[2:]:
            out_ls[adj-1] = 1

    print(*out_ls)
