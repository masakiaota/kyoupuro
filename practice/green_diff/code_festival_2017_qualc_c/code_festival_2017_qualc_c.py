# https://atcoder.jp/contests/code-festival-2017-qualc/tasks/code_festival_2017_qualc_c
# xを抜いたときにその文章が回文になっていれば必ず回文にすることができる
# 回分にできる場合は両端から、文字をマッチングさせていけばxを挿入する最小回数が求められる

# シンプルな問題なのに実装に時間をかけ過ぎだな

enu = enumerate
ra = range
s = input()
s_nx = []
for ss in s:
    if ss != 'x':
        s_nx.append(ss)

if s_nx != s_nx[::-1]:
    print(-1)
    exit()

n = (len(s_nx) + 1) // 2  # 真ん中の文字は何文字目か(1based)


# どうやったらバグらせにくい考えよう
# 各文字間に含まれるxの数をカウントして配列に入れる。あとはその配列を逆さにして差を取れば良い
s = s
x_cnt = [0]
for ss in s:
    if ss != 'x':
        x_cnt.append(0)
    else:
        x_cnt[-1] += 1
ans = 0
for a, b in zip(x_cnt, reversed(x_cnt)):
    ans += abs(a - b)


# print(x_cnt)
print(ans // 2)
