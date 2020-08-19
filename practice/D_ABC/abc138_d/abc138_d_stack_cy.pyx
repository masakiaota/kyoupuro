# 木上の累積和

import sys
from collections import defaultdict
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))


cdef ints():
    return list(map(int, read().split()))

cdef:
    long N,Q,a,b, q,x,_
    long cnt[200005]

N, Q = ints()
tree=[[] for _ in range(N)]
for _ in range(N - 1):
    a, b = mina(*ints())
    tree[a].append(b)
    tree[b].append(a)

for i in range(Q):
    q, x = ints()
    cnt[q - 1] += x

# dfsでcntに木に沿った累積和をsetしていく

cdef dfs(int u, int p):  # 戻り値なしver
    S_args = [(u, p)]  # 引数管理のstack
    S_cmd = [0]  # 0:into, 1:outofの処理をすべきと記録するstack

    def into(args):
        '''入るときの処理'''
        u, p = args
        cnt[u] += cnt[p]

    def nxt(args):
        S_args.append(args)  # 抜けるときに戻ってくることを予約
        S_cmd.append(1)
        '''今の引数からみて次の引数を列挙'''
        u, p = args
        cdef int nx
        for nx in tree[u]:
            if nx == p:
                continue
            _stack(nx, u)

    def outof(args):
        '''抜けるときの処理'''
        pass

    def _stack(*args):  # お好きな引数で
        S_args.append(args)
        S_cmd.append(0)

    while S_cmd:
        now_args = S_args.pop()
        cmd = S_cmd.pop()
        if cmd == 0:
            into(now_args)
            nxt(now_args)  # 次の再帰する(次のintoを予約)
        else:
            outof(now_args)


dfs(0, N)
for i in range(N):
    print(cnt[i], end=' ')
