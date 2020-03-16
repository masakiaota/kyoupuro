# https://atcoder.jp/contests/abc043/submissions/10933592
# 性質を考察すると、アンバランス十分条件は3つに2つ同じ文字が存在することである
s = input()
if len(s) == 2:
    if s[0] == s[1]:
        print(1, 2)
    else:
        print(-1, -1)
else:
    for i, (a, b, c) in enumerate(zip(s, s[1:], s[2:])):
        tmp = set([a, b, c])
        if len(tmp) != 3:
            print(i + 1, i + 3)
            exit()
    print(-1, -1)
