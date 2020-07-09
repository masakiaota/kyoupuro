# https://atcoder.jp/contests/abc003/tasks/abc003_2
for a, b in zip(input(), input()):
    if a != b and (('@' not in [a, b])
                   or (a == '@' and b not in 'atcoder')
                   or (b == '@' and a not in 'atcoder')):
        print('You will lose')
        exit()
print('You can win')
