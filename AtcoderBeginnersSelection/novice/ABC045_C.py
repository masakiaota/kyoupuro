S = input()

n = len(S) - 1
ans = ''

for i in range(1 << n):
    tmp = S[0]
    for j in range(n):
        if (i >> j) % 2:
            # もしビットが1だったら＋ことにする
            tmp += '+'+S[j+1]
        else:
            tmp += S[j+1]
    ans += '+'+tmp


print(eval(ans[1:]))
