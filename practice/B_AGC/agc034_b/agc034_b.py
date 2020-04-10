# https://atcoder.jp/contests/agc034/tasks/agc034_b

# BCをDに置換すればあとは簡単。いくつのAがDを超えるのかをカウントするだけ
# ADとなる
s = input()
s = s.replace('BC', 'D')
# print(s)

nA = 0
ans = 0

for ss in s:
    if ss == 'A':
        nA += 1
    elif ss == 'D':
        ans += nA
    else:
        nA = 0
print(ans)
