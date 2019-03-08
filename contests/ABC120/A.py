def readln():
    return list(map(int, input().split()))


A, B, C = readln()
if B // A > C:
    print(C)
else:
    print(B//A)
