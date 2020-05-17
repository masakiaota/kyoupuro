import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


import numpy as np

A, B, H, M = read_ints()

thetaA = (H + M / 60) / 12 * 2 * np.pi
thetaB = (M / 60) * 2 * np.pi
pA = A * np.exp(complex(0, thetaA))
pB = B * np.exp(complex(0, thetaB))

print(np.abs(pA - pB))
