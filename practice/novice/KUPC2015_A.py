# 問題 https://atcoder.jp/contests/kupc2015/tasks/kupc2015_a

T = int(input())
S = [input() for _ in range(T)]

for s in S:
    # 個々の処理
    # kyoto or tokyo が完成した時点で切るのが最適
    ans = 0
    i = 0
    s_len = len(s)
    while i + 5 <= s_len:

        if s[i:i + 5] == 'kyoto' or s[i:i + 5] == 'tokyo':
            ans += 1
            i += 5
        else:
            i += 1

    print(ans)
