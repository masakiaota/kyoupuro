from collections import Counter


def readln():
    return list(map(int, input().split()))


N = readln()[0]
S = input()
mod = 10**9+7

# 下記のコメントアウトした処理はこの処理で代用できる。
dic = Counter(S)
#dic = defaultdict(lambda: 0)
# for s in S:
#     dic[s] += 1
ans = 1
for count in dic.values():
    # print(count)
    ans *= (count + 1)
    ans %= mod

# print(ans)
print(ans-1)
