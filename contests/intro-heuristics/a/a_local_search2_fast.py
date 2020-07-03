# large n_trials is all you need
# local searchをたくさんするためにcost計算のさらなる高速化を考えてみる
# online judgeの環境ではサチるほど探索されていたみたいで改善されなかった

from time import time
t0 = time()

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right, insort_left
from functools import reduce
from random import randint, random

D = a_int()
C = ints()
S = read_tuple(D)


def T_to_date_by_contest(T):
    '''Tを日付形式にしつつscoreも計算'''
    date_by_contest = [[-1] for _ in range(26)]
    for d, t in enumerate(T):
        date_by_contest[t].append(d)
    for i in range(26):
        date_by_contest[i].append(D)  # 番兵
    return date_by_contest


def eval(D, C, S, date_by_contest):
    '''2~3*D回のループでスコアを計算する'''
    score = 0
    for c, dates in enu(date_by_contest):
        for d in dates[1:-1]:
            score += S[d][c]
        for i in range(len(dates) - 1):
            dd = (dates[i + 1] - dates[i])
            # for ddd in range(dd):
            #     score -= C[c] * (ddd)
            score -= C[c] * (dd - 1) * dd // 2
    return score


def maximizer(newT, bestT, bestscore):
    '''具体的なTの最大化用'''
    tmpscore = eval(D, C, S, T_to_date_by_contest(newT))
    if tmpscore > bestscore:
        return newT, tmpscore
    else:
        return bestT, bestscore


def ret_init_T():
    '''greedyで作ったTを初期値とする。
    return
    ----------
    T, score ... 初期のTとそのTで得られるscore
    '''
    def _make_T(n_days):
        # editorialよりd日目の改善は、改善せずにd+n_days経過したときの関数にしたほうが
        # 最終的なスコアと相関があるんじゃない？
        T = []
        last = [-1] * 26
        for d in range(D):
            ma = -INF
            for i in range(26):
                tmp = S[d][i]
                dd = d - last[i]
                tmp += C[i] * (((dd + n_days + dd) * (n_days) // 2))
                if tmp > ma:
                    t = i
                    ma = tmp
            last[t] = d  # Tを選んだあとで決める
            T.append(t)
        return T
    T = _make_T(2)
    sco = eval(D, C, S, T_to_date_by_contest(T))
    for i in range(3, 16):
        T, sco = maximizer(_make_T(i), T, sco)
    return T, sco


class Schedule:
    def __init__(self, T: list, date_by_contest, score: int):
        self.T = T
        self.date_by_contest = date_by_contest
        self.score = score

    def try_change_contest(self, d, j):
        '''d日目をjに変更したときのscore'''
        score = self.score
        i = self.T[d]  # コンテストi→jに変化する
        if i == j:
            return score  # 変化しないので
        score += S[d][j] - S[d][i]

        # iの変化についてscoreを計算し直す
        # d_i_idx = bisect_left(self.date_by_contest[i], d)  # iにおけるdのindex
        d_i_idx = self.date_by_contest[i].index(d)  # iにおけるdのindex
        dd = self.date_by_contest[i][d_i_idx + 1] - \
            self.date_by_contest[i][d_i_idx - 1]
        score -= C[i] * (dd - 1) * dd // 2
        dd = self.date_by_contest[i][d_i_idx + 1] - d
        score += C[i] * (dd - 1) * dd // 2
        dd = d - self.date_by_contest[i][d_i_idx - 1]
        score += C[i] * (dd - 1) * dd // 2
        # jの変化についてscoreを計算し直す
        d_j_idx = bisect_left(self.date_by_contest[j], d)
        dd = self.date_by_contest[j][d_j_idx] - \
            self.date_by_contest[j][d_j_idx - 1]
        score += C[j] * (dd - 1) * dd // 2
        dd = self.date_by_contest[j][d_j_idx] - d
        score -= C[j] * (dd - 1) * dd // 2
        dd = d - self.date_by_contest[j][d_j_idx - 1]
        score -= C[j] * (dd - 1) * dd // 2
        return score

    def change_contest(self, d, j):
        '''d日目をjに変更する'''
        self.score = self.try_change_contest(d, j)
        i = self.T[d]
        self.T[d] = j
        self.date_by_contest[i].remove(d)
        insort_left(self.date_by_contest[j], d)


def trial(sche, thre_p, days_near):
    '''確率的にどちらかの操作を行ってよかったらScheduleを更新する
    1.日付dとコンテストqをランダムに選びd日目に開催するコンテストのタイプをqに変更する
    2.10日以内の点でコンテストを入れ替える
    thre_pはどちらの行動を行うかを調節、days_nearは近さのパラメータ'''
    if random() < thre_p:
        # 一点更新
        d = randint(0, D - 1)
        q = randint(0, 25)
        if sche.score < sche.try_change_contest(d, q):
            sche.change_contest(d, q)
        return sche  # 参照渡しだから変わらんけどね
    else:
        T = sche.T.copy()
        i = randint(0, D - 2)
        j = randint(i - days_near, i + days_near)
        j = max(j, 0)
        j = min(j, D - 1)
        if i == j:
            j += 1
        T[i], T[j] = T[j], T[i]
        new_score = eval(D, C, S, T_to_date_by_contest(T))
        if sche.score < new_score:
            return Schedule(T, T_to_date_by_contest(T), new_score)
        else:
            return sche


bestT, bestscore = ret_init_T()
sche = Schedule(bestT, T_to_date_by_contest(bestT), bestscore)

# while time() - t0 < 1.90:
while time() - t0 < 5:
    for _ in range(1000):
        sche = trial(sche, 0.9, 20)

# print(sche.score)
# print(score(D, C, S, T))
print(*mina(*sche.T, sub=-1), sep='\n')
