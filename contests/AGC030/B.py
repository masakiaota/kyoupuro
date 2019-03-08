L, N = list(map(int, input().split()))
X = [int(input()) for _ in range(N)]

X = [x - L for x in X] + X

ans = 0
taka = 0


def search_longer(now, pre, nex):
    # print((nex - now), (now - pre))
    if (nex - now) > (now - pre):
        return nex - now, True
    elif (nex - now) < (now - pre):
        return now-pre, False


while (len(X) != 0):
    nex = int(len(X) / 2)
    # print("nex", nex)
    dist, isnex = search_longer(taka, X[nex - 1], X[nex])
    ans += dist
    # print(taka, dist, ans, isnex)
    # print(X)
    if isnex:
        taka = X[nex]
        X.pop(nex)
        X.pop(0)
    else:
        taka = X[nex-1]
        X.pop(nex - 1)
        X.pop(-1)

print(ans)
# 直近1つを見ただけではどうやら違うっぽい
