# hyper parameter tuning is all you need
import optuna
from random import randrange, random, randint, seed
from time import time
from math import exp
from bisect import bisect_left, insort_left
import joblib
from numba import jit, njit, jitclass
INF = 2**31  # 2147483648 > 10**9
from multiprocessing import Pool
import multiprocessing as multi
from functools import partial


@njit
def generate_DCS():
    # seed(random_state)
    D = 365
    c = [randrange(0, 101) for _ in range(26)]
    s = [[randrange(0, 20001) for _ in range(26)] for _ in range(D)]
    return D, c, s


@njit
def annealing(oldscore, newscore, T):
    '''p(newscore-oldscore,T)=min(1,exp((newscore-oldscore)/T)) の確率でnewscoreを採用する
    newが選ばれた時はTrueを返す'''
    if oldscore < newscore:
        return True
    else:
        p = exp((newscore - oldscore) / T)
        return random() < p


dataset = [generate_DCS() for _ in range(50)]


def ret_one_trialscore(i, T0, T1, TL, thre_p, days_near):
    # D, C, S = generate_DCS(random_state)
    D, C, S = dataset[i]
    t0 = time()

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
        for c, dates in enumerate(date_by_contest):
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

    def trial(sche, thre_p, days_near, Tt):
        '''確率的にどちらかの操作を行ってよかったらScheduleを更新する
        1.日付dとコンテストqをランダムに選びd日目に開催するコンテストのタイプをqに変更する
        2.10日以内の点でコンテストを入れ替える
        thre_pはどちらの行動を行うかを調節、days_nearは近さのパラメータ'''
        if random() < thre_p:
            # 一点更新
            d = randint(0, D - 1)
            q = randint(0, 25)
            if annealing(sche.score, sche.try_change_contest(d, q), Tt):
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
            if annealing(sche.score, new_score, Tt):
                return Schedule(T, T_to_date_by_contest(T), new_score)
            else:
                return sche

    bestT, bestscore = ret_init_T()
    sche = Schedule(bestT, T_to_date_by_contest(bestT), bestscore)

    now = time()
    while now - t0 < TL:
        Tt = (T0**((now - t0) / TL)) * (T1**(1 - (now - t0) / TL))
        for _ in range(3000):
            sche = trial(sche, thre_p, days_near, Tt)
        now = time()

    return sche.score


def objective(trial):
    T0 = trial.suggest_loguniform('T0', 10, 10**3)
    # T0 = 200
    T1 = 5
    TL = 25
    thre_p = trial.suggest_uniform('thre_p', 0.5, 1)
    # thre_p = 0.9
    days_near = trial.suggest_int('days_near', 1, 50)
    # days_near = 20
    process = partial(ret_one_trialscore, T0=T0, T1=T1,
                      TL=TL, thre_p=thre_p, days_near=days_near)
    pool = Pool(multi.cpu_count() - 2)
    results = pool.map(process, list(range(40)))
    pool.close()
    # ret = sum(10**6 + ret_one_trialscore(i, T0, T1, TL, thre_p, days_near)
    #          for i in range(50))
    ret = sum(results) * 5 / 4 + 50 * 10**6
    print(ret, trial.params)
    return -ret


study = optuna.create_study()
# storage='sqlite:///example.db', load_if_exists=True)
study.optimize(objective, timeout=60 * 60 * 9, n_jobs=1)
joblib.dump(study, 'unko_study.jb', compress=3)

print(study.best_params)
