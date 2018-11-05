#import numpy as np
N = int(input())
S = input()

left, right = ([0] * 2)

for kakko in S:
    if kakko is "(":
        left += 1
    elif kakko is ")":
        if left is not 0:
            left -= 1
        else:
            right += 1

print("("*right+S+")"*left)
