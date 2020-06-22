# https://atcoder.jp/contests/agc022/tasks/agc022_a
# いままで登場してない文字で最小のものをappendすれば良い
# ただし26文字すべて出現している場合には考える必要あり
# つまり後ろからi文字目はそれ以降に出現した文字を使って一つだけ大きくできれば良い

S = input()
abc = list(reversed('zyxwvutsrqponmlkjihgfedcba'))

if S == 'zyxwvutsrqponmlkjihgfedcba':
    print(-1)
elif len(S) < 26:
    for s in S:
        abc.remove(s)
    print(S + abc[0])
else:
    canuse = set()
    for i in range(25, -1, -1):
        canuse.add(S[i])
        add = None
        for can in sorted(canuse):
            if can > S[i]:
                add = can
                break
        if add != None:
            print(S[:i] + add)
            exit()
