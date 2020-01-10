# https://atcoder.jp/contests/abc140/tasks/abc140_d

# 性質ちゃんと考える
# 連長圧縮したあとならば、r-lが奇数である組み合わせをひっくり返すのに価値がある
# 反転しても中の幸福度は変化しない。
# 幸福度が改善するのはLR or RLでなくなった部分だけ

# LLLLLL (幸福度5(max))
# LRLRRL (幸福度1 (4人死んでるので)) #一回の操作が最善ならば必ず2人改善する(l==rの場合は1人改善)
# LLRLLL (幸福度3 (1から2人復活したので))


# もしラストをひっくり返すことで幸福度が上がるなら？
# ラストに0文字のLをfillする必要がある？
# LRRLRLRRLRLLR (max 12) (N-1) (今は3)
# 3回操作によって＋6なので9となる

# 10 1
# LLLLLRRRRR (max9) (いま8)
# そうかL→RとR→Lの個数をカウントすればいいんだ

N, K = list(map(int, input().split()))
S = input()

score = -1
LtoR = 0
RtoL = 0
pre = S[0]
for s in S:
    if pre == s:
        score += 1
    elif pre == 'L':
        LtoR += 1
    elif pre == 'R':
        RtoL += 1
    pre = s

# print(score, LtoR, RtoL)
if K <= min(LtoR, RtoL):
    print(score + 2 * K)
else:
    print(score + LtoR + RtoL)
