# https://atcoder.jp/contests/abc113/tasks/abc113_d
# 1からスタートKまで到着
# 横棒はH本引ける
# 縦棒はW本ある

# DP的に数え上げることを考える
# DP[i][j]...(0,i]まで考慮したときに、jにたどり着ける通りの数
MOD = 10**9 + 7
ra = range
enu = enumerate
H, W, K = map(int, input().split())


'''
DP[i][j]...(0,i]まで考慮したときに、jにたどり着ける通りの数

更新則
if iにおいてjとj+1の間に横棒があったら
DP[i+1][j+1] += D[i][j]
DP[i+1][j] += D[i][j+1]
else
DP[i+1][j] = D[i][j]

for all paterns at i

初期条件
DP[i][j]=0
DP[0][0]=1
'''

if W == 1:  # コーナーケース処理
    print(1)
    exit()


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


def ret_valid_swapset():
    # (j,j+1)でswapできるならjで1に成るようにする
    for p in iter_p_adic(2, W - 1):
        pre = 0
        skip = False
        for pp in p:
            if pre == 1 and pp == 1:
                skip = True
                break  # 条件を満たさない
            pre = pp
        if skip:
            continue
        yield p


pattern = list(ret_valid_swapset())

DP = [[0] * W for _ in ra(H + 1)]
DP[0][0] = 1
for i in ra(H):
    for p in pattern:
        pre_is_swap = False
        for j, is_swap in enu(p):
            if is_swap:
                DP[i + 1][j + 1] += DP[i][j]
                DP[i + 1][j] += DP[i][j + 1]
            # swapしない線の処理
            elif not pre_is_swap:
                DP[i + 1][j] += DP[i][j]
            pre_is_swap = is_swap
        if not pre_is_swap:  # 一番端の処理も忘れずに
            DP[i + 1][j + 1] += DP[i][j + 1]

    # あまりの処理
    for j in ra(W):
        DP[i + 1][j] %= MOD
print(DP[H][K - 1])
# from pprint import pprint
# print(*DP, sep='\n')
