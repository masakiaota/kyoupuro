import sys
read = sys.stdin.readline


def readln():
    return list(map(int, read().split()))


N, Q = readln()
S = input()

L = []
R = []
for _ in range(Q):
    a, b = readln()
    L.append(a-1)  # indexでアクセスできるように
    R.append(b-1)

AC_count = [0] * N  # Sのidxと対応、そこまでにACが何回出てきたか
cnt = 0

for i, (s1, s2) in enumerate(zip(S[:-1], S[1:])):
    if s1 == 'A' and s2 == 'C':
        cnt += 1

    AC_count[i+1] = cnt

for l, r in zip(L, R):
    print(AC_count[r]-AC_count[l])
