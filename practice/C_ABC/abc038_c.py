# https://atcoder.jp/contests/abc038/tasks/abc038_c

# 可能な限り伸ばした部分単調増加列の長さをnとしたとき、
# l,rの選び方は重複を許さずに選ぶ通りの数は nC2
# 更にl==rである場合を許すのだから nC2 + n がその最長部分単調増加列に含まれる部分単調増加列

N = int(input())
A = list(map(int, input().split()))

ans = 0
cnt = 1
pre = A[0]
for a in A[1:]:
    if a > pre:
        cnt += 1
    else:
        ans += cnt * (cnt - 1) // 2 + cnt
        cnt = 1
    pre = a

# 終端処理
ans += cnt * (cnt - 1) // 2 + cnt

print(ans)
