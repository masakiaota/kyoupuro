# https://atcoder.jp/contests/abc128/tasks/abc128_c
# なんかかすかに解いた記憶あるな？

# load data
N, M = list(map(int, input().split()))
S = []
for _ in range(M):
    S.append(list(map(int, input().split()))[1:])
P = list(map(int, input().split()))


def is_on(i, S, p):
    # bit表現 i のときSのスイッチの個数とpでonか返す
    cnt = 0
    for j in S:
        if (i >> (j - 1)) % 2:  # j番目のスイッチオン
            cnt += 1
    return cnt % 2 == p


ans = 0
# bit 全探索
for i in range(1 << N):  # N桁のbit全探索
    # 以下iの全パターンについて
    flg = True
    for s, p in zip(S, P):
        if not is_on(i, s, p):
            flg = False
            break
    ans += flg
print(ans)
