def readln():
    return list(map(int, input().split()))


H, W = readln()
h, w = readln()
print((H-h)*(W-w))
