# https://atcoder.jp/contests/abc019/tasks/abc019_4
# dfsで一番深いノードを二回探すと木の直径がわかる。それを回答でやってやれば良い
import sys
N = int(input())

# 1から一番遠いノードは？
dist = 0
otherside = -1
for i in range(2, N + 1):
    print('? 1', i)
    sys.stdout.flush()
    res = int(input())
    if dist < res:
        dist = res
        otherside = i

# othersideから一番通りのが木の直径
ans = 0
for i in range(1, N + 1):
    if i == otherside:
        continue
    print('?', otherside, i)
    sys.stdout.flush()
    ans = max(ans, int(input()))
print('!', ans)
