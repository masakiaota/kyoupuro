# https://atcoder.jp/contests/arc003/tasks/arc003_2
N = int(input())
words = []
for _ in range(N):
    s = input()
    words.append((s[::-1], s))

words.sort()
for _, w in words:
    print(w)
