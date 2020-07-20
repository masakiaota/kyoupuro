# https://atcoder.jp/contests/arc019/tasks/arc019_2
# 名前は何通り作れるか？→len(S)*25 通り
# 回文にするには何通り作れるか？ → 0通り or 1通り or 26通りでは？
# 回文のすべての対について、一つだけ不一致である単語は、今回回文になりうる


S = input()
l = len(S)
cnt = 0  # 成り立たない対の数
for i in range(l // 2):
    cnt += S[i] != S[-i - 1]

if cnt >= 2:
    # 2つ以上の対が成り立たない→何しても回分じゃない
    print(l * 25)
elif cnt == 1:
    # 1つの対が成り立たない→その対だけ場合分けで数え上げ
    print((l - 2) * 25 + 48)
else:
    # 最初から回分だった場合 奇数の場合のみ真ん中を動かしても回分になってしまうことに注意
    print(l * 25 - (25 if len(S) & 1 else 0))
    # print((l - (l & 1)) * 25)
