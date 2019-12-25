# https://atcoder.jp/contests/abc136/tasks/abc136_c
# 前から順に最善を考えていけば良い

N = int(input())
H = list(map(int, input().split()))


# 1 2 1 1 3
# 0 1 1 1 2 みたいにして判別する

# 1 3 1 1 3
# 0 2 1 #この時点でだめ

pre = H[0] - 1  # 一段下げても`悪化しない`
for h in H[1:]:
    if h > pre:
        pre = h - 1
    elif h == pre:
        pre = h
    elif h < pre:
        print('No')
        exit()

print('Yes')
