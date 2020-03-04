# https://atcoder.jp/contests/abc071/tasks/arc081_b
# 横ドミノと縦ドミノの場合わけを考える
# 横ドミノ→横ドミノの場合 : いままでの通り*3通り
# 横ドミノ→縦ドミノ : いままで*1通り ok
# 縦ドミノ→横ドミノ : いままで*2通り ok
# 縦ドミノ→縦ドミノ : いままで*2通り ok

MOD = 10**9 + 7
N = int(input())
S1 = input()
S2 = input()
pre1 = S1[0]
pre2 = S2[0]

if pre1 != pre2:
    ans = 6
else:
    ans = 3
for s1, s2 in zip(S1[1:], S2[1:]):
    if pre1 == s1 and pre2 == s2:
        pass
    elif pre1 != pre2 and s1 != s2:
        # 横→横
        ans *= 3
    elif pre1 != pre2 and s1 == s2:
        # 横→縦
        pass
    elif pre1 == pre2 and s1 != s2:
        # 縦→横
        ans *= 2
    elif pre1 == pre2 and s1 == s2:
        # 縦→縦
        ans *= 2

    if ans >= MOD:
        ans %= MOD

    pre1, pre2 = s1, s2


print(ans % MOD)
