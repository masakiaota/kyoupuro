from typing import List


def idx_last_one(rows: List[str]):
    ret = []
    for s in rows:
        last = -1
        for i in range(len(s)):
            if s[i] == '1':
                last = i
        ret.append(last)
    return ret


def solve(N: int, idxs_last_one: List[int]):
    ans = 0
    # 愚直にswapする
    for i in range(N):
        # i行目については last_idx<=iであるもののうち一番近いものを使うのが最適
        for j in range(i, N):
            last_idx = idxs_last_one[j]
            if last_idx <= i:
                break
        ans += j - i  # swapする回数は計算可能
        del idxs_last_one[j]
        idxs_last_one.insert(i, last_idx)
    print(ans)


# 入力例1
N = 2
mat = ['10',
       '11', ]
solve(N, idx_last_one(mat))

# 入力例3
N = 3
mat = ['001',
       '100',
       '010', ]
solve(N, idx_last_one(mat))


# 入力例3
N = 4
mat = ['1110',
       '1100',
       '1100',
       '1000']
solve(N, idx_last_one(mat))
