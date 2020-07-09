# https://atcoder.jp/contests/abc039/tasks/abc039_c

kenban = 'WBWBWWBWBWBW' * 2
ans = ['Do', '', 'Re', '', 'Mi', 'Fa', '', 'So', '', 'La', '', 'Si']
S = input()[:12]
for i in range(12):
    if kenban[i:i + 12] == S:
        print(ans[i])
        exit()
