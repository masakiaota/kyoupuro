# https://atcoder.jp/contests/abc042/tasks/arc058_a
# 最悪でも10**6探索する程度なのでNを一つづつ足して全探索でもよくね？


N, K = list(map(int, input().split()))
D = set(input().split())


def is_inD(n: int):
    tmp = set(str(n))
    return tmp.isdisjoint(D)


# 頭の悪い全探索
for i in range(N, N + 10 ** 6):
    if is_inD(i):
        print(i)
        exit()


# もっとスケーラブルな方法 #バグらせまくった #わからん
# all_nums = {str(i) for i in range(10)}
# use_nums = all_nums - D
# N_str = '0' + str(N)
# L = len(N_str)

# ans = []
# for l in range(L):
#     tmp = []
#     for num in use_nums:
#         if int(num) >= int(N_str[l]):
#             tmp.append(num)
#     ans.append(min(tmp))

# print(int(''.join(ans)))
