x, y = list(map(int, input().split()))

gA = [1, 3, 5, 7, 8, 10, 12]
gB = [4, 6, 9, 11]
gC = [2]

flg = False
for g in [gA, gB, gC]:
    if x in g:
        if y in g:
            print("Yes")
            flg = True

if not flg:
    print("No")
