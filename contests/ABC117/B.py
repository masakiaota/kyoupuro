N = int(input())
L = list(map(int, input().split()))

l_max = max(L)
L.remove(l_max)

if sum(L) <= l_max:
    print('No')
else:
    print('Yes')
