# https: // onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/4/ALDS1_4_A
# やるだけ # 本では番兵を用いた実装だと定数倍早いとあるが、pythonではwhileが遅いのでおとなしくfor文を書く

n = input()
S = list(map(int, input().split()))
q = input()
T = list(map(int, input().split()))

cnt = 0
for t in T:
    for s in S:
        if t == s:
            cnt += 1
            break
print(cnt)
