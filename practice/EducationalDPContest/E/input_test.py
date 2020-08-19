
import sys
read = sys.stdin.buffer.read
readline = sys.stdin.buffer.readline
readlines = sys.stdin.buffer.readlines

import numpy as np


# print(np.frombuffer(read().rstrip(), dtype=np.int64))
# print(np.fromstring('1 4 6', dtype=np.int64, sep=' '))
# print(read())
print(readline())
print()
print(readlines(1000000))
