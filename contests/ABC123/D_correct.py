# pypy3だと通ったがpython3だとTLE

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


X, Y, Z, K = read_ints()
A = read_ints()
B = read_ints()
C = read_ints()

from itertools import product

AB = [a + b for a, b in product(A, B)]
AB.sort(reverse=True)

ABC = [ab + c for ab, c in product(AB[:K], C)]
ABC.sort(reverse=True)

ABC = ABC[:K]

print(*ABC, sep='\n')
