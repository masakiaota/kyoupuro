# https://atcoder.jp/contests/arc008/tasks/arc008_2

from collections import Counter
N, M = map(int, input().split())
name = input()
kit = input()


# floor(必要なの文字数/キットの文字数)の最大が答え
cnt_name = Counter(name)
cnt_kit = Counter(kit)
ans = 0
for k, v in cnt_name.items():
    if cnt_kit[k] == 0:
        print(-1)
        exit()
    ans = max(ans,
              (v - 1) // cnt_kit[k] + 1)

print(ans)
