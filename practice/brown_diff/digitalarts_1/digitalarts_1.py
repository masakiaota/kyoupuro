# https://atcoder.jp/contests/digitalarts2012/tasks/digitalarts_1
# まあ愚直でええか
s = input().split()
N = int(input())
T = []
for _ in range(N):
    T.append(input())


def is_detect(target: str, source: str):
    if len(target) != len(source):
        return False
    for t, s in zip(target, source):
        if t == '*':
            continue
        if t != s:
            return False
    return True


ans = []
for ss in s:
    is_ban = False
    for t in T:
        if is_detect(t, ss):
            is_ban = True
            break
    if is_ban:
        ans.append('*' * len(ss))
    else:
        ans.append(ss)
print(*ans)
