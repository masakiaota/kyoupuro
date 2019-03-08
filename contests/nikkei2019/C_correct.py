N = int(input())
# 仮に青木さん(B)が全て食べたとき、高橋(A)にとっては最悪な状況
# これを打破するためには、自分の得点と相手の得るはずだった得点の和を最大にするようなものについて食べれば良い
# 青木さんも同じことを考えるので、要は幸福度の和をソートして交互に食べていけば、互いに最善の行動を取ることとなる。
# 答えを出す際には、AがBから何点奪ったかだけを考えればよい。
# つまり、Aが最善で何点奪えるか　から　全てBさんが食べたとき　を引けばいい

sum_happiness = []
all_for_B = 0

for i in range(N):
    tmp1, tmp2 = list(map(int, input().split()))
    sum_happiness.append(tmp1 + tmp2)
    all_for_B += tmp2

sum_happiness.sort(reverse=True)


gain_A = 0
for i, h in enumerate(sum_happiness):
    if i % 2 == 0:
        gain_A += h
    # print(gain_A)


print(gain_A - all_for_B)
