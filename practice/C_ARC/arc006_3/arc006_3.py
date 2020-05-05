# https://atcoder.jp/contests/arc006/tasks/arc006_3
# 山の最上段だけ覚えておく
# 自分以上の数字の中で、一番自分との差が小さい数字を自身と入れ替える
# Nは50しかないのでいちいちソートしても線形探索しても普通に間に合う！


N = int(input())
yama = []
for _ in range(N):
    w = int(input())
    yama.sort()
    for i, y in enumerate(yama):
        if w <= y:  # 初めて自分以上の山がある
            idx = i
            break
    else:
        idx = -1
    if idx == -1:
        yama.append(w)
    else:
        yama[i] = w


# print(yama)
print(len(yama))
