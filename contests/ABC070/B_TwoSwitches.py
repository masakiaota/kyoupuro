A, B, C, D = list(map(int, input().split()))

if A < C:
    if B - C > 0:
        print(min(B - C, B - A, D - C))
    else:
        print("0")
elif C < A:
    if D - A > 0:
        print(min(D - A, B - A, D - C))
    else:
        print("0")
else:
    print(min(B - A, D - C))
