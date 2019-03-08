N = int(input())
T = []
for i in range(N):
    T.append(int(input()))

# 要は最小公倍数っぽいものを求めろ
import numpy as np
T = np.array(T)

# 連除法を使う
ans = 1

flg = True
while(flg):
    for i in range(2, int(np.sqrt(max(T))) + 1):
        if sum(T % i) == 0:
            T = T // i
            # print(T)
            ans = ans * i
            break
        flg = False

for t in np.unique(T):
    ans = ans * t

# 処理がおそすぎて終わらん…
print(ans)
