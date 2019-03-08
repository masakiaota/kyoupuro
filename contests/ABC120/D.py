from itertools import combinations


def readln():
    return list(map(int, input().split()))


N, M = readln()
AB = [readln() for _ in range(M)]
A = [ab[0] for ab in AB]
B = [ab[1] for ab in AB]

fuben_list = [conbi for conbi in combinations(range(N), 2)]
max_fuben = len(fuben_list)

ans = [max_fuben]
set_list = [{A[-1], B[-1]}]
for a, b in zip(A[::-1], B[::-1]):
    flg = []
    for i, s in enumerate(set_list):
        if a in s:
            flg.append(i)
            set_list[i] = set_list[i] | {b}
        elif b in s:
            flg.append(i)
            set_list[i] = set_list[i] | {a}
        else:
            set_list.append({a, b})
        if len(flg) == 2:
            set_list[min(flg)] = set_list[flg[0]] | set_list[flg[1]]
            set_list.remove(set_list[max(flg)])
    tmp = 0
    for s in set_list:
        tmp += len(list(combinations(s, 2)))
    ans.append(max_fuben-tmp)

for i, a in enumerate(ans[::-1]):
    if i == 0:
        continue
    print(a)
