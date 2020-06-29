# https://atcoder.jp/contests/arc080/tasks/arc080_a
N = int(input())
A = list(map(int, input().split()))

# 2の倍数ならばどのように並んでてもよい
# 4の倍数ならば隣り合うものは2の倍数でなくともよい

multi4 = 0
multi2 = 0
other = 0
for a in A:
    if a % 4 == 0:
        multi4 += 1
    elif a % 2 == 0:
        multi2 += 1
    else:
        other += 1

if multi2 == 0:
    print('Yes' if other <= multi4 + 1 else 'No')
else:
    print('Yes' if other <= multi4 else 'No')
