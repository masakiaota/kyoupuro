# https://atcoder.jp/contests/abc131/tasks/abc131_d
# 最適なのは締切が速い順から取り掛かること
import sys
from operator import itemgetter
read = sys.stdin.readline


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


N = read_a_int()
AB = read_tuple(N)
AB.sort(key=itemgetter(1))
jikoku = 0
for a, b in AB:
    jikoku += a
    if jikoku > b:
        print('No')
        exit()
print('Yes')
