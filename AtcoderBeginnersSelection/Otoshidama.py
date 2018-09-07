N, Y = list(map(int, input().split()))
Y /= 1000
flg = True
for man in range(N + 1):
    for gosen in range(N - man + 1):
        sen = N - man - gosen
        if (10 * man + 5 * gosen + sen) == Y:
            print("{} {} {}".format(man, gosen, sen))
            flg = False
            break
    if flg == False:
        break

if flg:
    print("-1 -1 -1")
