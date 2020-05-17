n = 5
a = [2, 3, 3, 5, 6]
k = 3

from bisect import bisect_left
print(bisect_left(a, k))
