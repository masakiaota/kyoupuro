# 問題 https://abc076.contest.atcoder.jp/tasks/abc076_c
S = input()
T = input()
Tlen = len(T)
Slen = len(S)
# Tの入りうる箇所の探索
# Tは入りうる箇所の一番うしろに入れて
# 前の?をすべてaで埋め尽くせばよい

for i in range(Slen-Tlen, -1, -1):
    # print(i)
    for s, t in zip(S[i:i + Tlen], T):
        flg = True
        if s != t and s != '?':
            flg = False
            break
    if flg:
        insertidx = i
        break
else:
    print('UNRESTORABLE')
    # import sys
    exit()

ans = S[:insertidx] + T + S[insertidx + Tlen:]
ans = ans.replace('?', 'a')
print(ans)
