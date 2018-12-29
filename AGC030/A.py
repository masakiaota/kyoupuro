A, B, C = list(map(int, input().split()))

if A + B >= C - 1:
    print(B + C)
else:
    print(A+2*B+1)
