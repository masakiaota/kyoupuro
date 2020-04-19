# https://atcoder.jp/contests/arc022/tasks/arc022_1
S = input().lower()

cnt = ['t', 'c', 'i']
for s in S:
    if s == cnt[-1]:
        cnt.pop()
        if len(cnt) == 0:
            print('YES')
            exit()
print("NO")
