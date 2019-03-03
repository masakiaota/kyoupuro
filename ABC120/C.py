S = input()

num_1 = 0
for s in S:
    if s is '1':
        num_1 += 1

num_0 = len(S) - num_1

print(len(S)-abs(num_0-num_1))
