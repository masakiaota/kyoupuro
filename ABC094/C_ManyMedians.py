N = int(input())
X = list(map(int, input().split()))
X_sort = X.copy()
X_sort.sort()
s = N // 2
median = (X_sort[s - 1] + X_sort[s]) / 2

for x in X:
    if x < median:
        print(X_sort[s])
    else:
        print(X_sort[s-1])
