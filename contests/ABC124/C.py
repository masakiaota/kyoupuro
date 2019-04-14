import sys
read = sys.stdin.readline

S = read()
len_s = len(S)-1
# print(S, len_s)
# ans0 = 0  # 0はじまり ->len_s-ans1で計算可能
ans1 = 0  # 1はじまり
for i, s in enumerate(S[:-1]):
    if i % 2:
        if s != '1':
            ans1 += 1
    else:
        if s != '0':
            ans1 += 1
    # print(i, ans1)

print(min(ans1, (len_s-ans1)))
