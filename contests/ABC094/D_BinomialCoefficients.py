n = int(input())
A = list(map(int, input().split()))
a_max = max(A)


def find_nearest(arr, target):
    m = min(arr)
    dist = abs(m-target)
    for a in arr:
        if abs(a-target) < dist:
            dist = abs(a - target)
            m = a
            # print(m, dist)
    return m


print(a_max, find_nearest(A, a_max/2))
