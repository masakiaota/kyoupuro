# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/4/ALDS1_4_B
# pythonでは便利な実装がすでにあるのでそれを使えるように
# 実装例はここにある https://docs.python.org/ja/3/library/bisect.html#searching-sorted-lists

from bisect import bisect_left
# 他にもbisect, bisect_leftがあったりするが、重複した値があったときに右側を返すか左側を返すかの違い
# bisectはrightと一緒


def find_exact_equal(a: list, x: int):
    i = bisect_left(a, x)
    if i != len(a) and a[i] == x:
        return 1  # 見つかったら1を返す。
    return 0  # 見つからなかったら0


n = input()
S = list(map(int, input().split()))
q = input()
T = list(map(int, input().split()))

cnt = 0
for t in T:
    cnt += find_exact_equal(S, t)

print(cnt)
