from itertools import product
N, Y = list(map(int, input().split()))
Y /= 1000
flg = True
for man, gosen, sen in product(range(N + 1), range(N + 1), range(N + 1)):
    if (10 * man + 5 * gosen + sen) == Y and (man + gosen + sen) == N:
        print("{} {} {}".format(man, gosen, sen))
        flg = False
        break

if flg:
    print("-1 -1 -1")
