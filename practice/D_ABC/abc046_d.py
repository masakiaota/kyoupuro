# https://atcoder.jp/contests/abc046/tasks/arc062_b
# 300点なのに水色diff
# AtCodeerくんの最善手はgpgpgpと交互に手を出すことである。任意のタイミングでpをgに変更したとき、負けることはあっても勝つことはない.
# (相手が何を出そうともpは負けることはない(悪化しない))

s = input()
ans = 0
for i, ss in enumerate(s):
    if i & 1:  # 自分はpである
        ans += ss == 'g'  # 相手がgだったら得点
    else:  # 自分はgである
        ans -= ss == 'p'  # 相手がpだったら減点
print(ans)
