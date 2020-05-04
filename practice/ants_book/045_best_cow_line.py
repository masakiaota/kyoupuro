# http://poj.org/problem?id=3617
N = 6
S = 'ACDBCB'

# Sの先頭か最後をTの末尾に追加しろ言っているのだから小さい方の文字を追加すれば良い
# ただし1文字比較では同じ文字だったときにバグるので文字列を保持して小さい方を選択する必要がある。
from collections import deque  # pythonはlist likeなオブジェクトの大小比較を辞書順でやってくれる
T = []
S_for = deque(S)
S_rev = deque(reversed(S))

while S_for and S_rev:
    if S_for <= S_rev:
        T.append(S_for.popleft())
        S_rev.pop()
    else:
        T.append(S_rev.popleft())
        S_for.pop()
print(''.join(T))
