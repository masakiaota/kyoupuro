# https://atcoder.jp/contests/abc041/tasks/abc041_c
# input()
# AID = [(a, i + 1) for i, a in enumerate(map(int, input().split()))]
# AID.sort(reverse=True)
# _, ID = zip(*AID)
# print(*ID, sep='\n')
input()
AID = sorted([(int(a), i + 1) for i, a in enumerate(input().split())])
_, ID = zip(*AID[::-1])
print(*ID, sep='\n')
