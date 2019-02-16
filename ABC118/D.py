N, M = list(map(int, input().split()))
A = list(map(int, input().split()))

# 変換表
num_to_n = {
    1: 2,
    2: 5,
    3: 5,
    4: 4,
    5: 5,
    6: 6,
    7: 3,
    8: 7,
    9: 6
}

A.sort()
n_to_num = {}
for a in A:
    n_to_num[num_to_n[a]] = a

set_of_n = [num_to_n[a] for a in A]
set_of_n = list(set(set_of_n))  # remove duplicate
# print(set_of_n)

# 桁数を大きく
# 書いててわけわからんくなってきた
n_cur = min(set_of_n)
n_rest = N
n_set = []
while (n_cur):
    syou, amari = divmod(n_rest, n_cur)
    while(amari not in set_of_n):
        syou -= 1
        amari += n_cur
    n_set.append(syou)
    n_rest = amari

# 最大桁はなるべく大きな数字になるように並べ替え
