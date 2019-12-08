# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/all/DPL_3_B
# この問題は前処理パートと、効率よく長方形の面積を求めるパートに分かれる。

from itertools import product, chain

# load data
H, W = list(map(int, input().split()))
C = []
for i in range(H):
    C.append(list(map(int, input().split())))


# まず、前処理を行う
# T (H,W) ... そのマスより上に何マスまで濡れるか
T = []
pre_tmp = [0] * W
for c in C:
    tmp = []
    for cc, pre in zip(c, pre_tmp):
        if cc == 1:
            tmp.append(0)
        else:
            tmp.append(pre + 1)
    T.append(tmp)
    pre_tmp = tmp.copy()


def get_largest_rectangle(hist: list):
    '''
    ヒストグラムが渡されたときに、その中の最大の長方形を返す
    '''
    hist = hist.copy()
    hist.append(0)
    ret = 0
    stack = []
    for i, v in enumerate(hist):
        # vが本で言うrectのこと
        if len(stack) == 0:
            stack.append((i, v))
        elif stack[-1][1] < v:
            stack.append((i, v))
        elif stack[-1][1] > v:
            while stack and stack[-1][1] > v:
                # スタックが空でなく、最後の要素がvより大きい限りは、
                # 面積の最大値を更新していく
                i_left, h = stack.pop()
                ret = max(ret, (i - i_left) * h)
            stack.append((i_left, v))
    return ret


# 各行について走査することで長方形の最大値を求める
ans = 0
for t in T:
    ans = max(ans, get_largest_rectangle(t))
print(ans)
