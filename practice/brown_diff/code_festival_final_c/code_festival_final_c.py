A = int(input())


def f(N, A):
    for i, Ni in enumerate(reversed(str(N))):
        if (int(Ni) != A % N) or A == 0:
            return False
        A //= N
    if A > 0:
        return False
    # Niを使い切るのとAが同時に0にならなければいけない
    return True


for k in range(10, 10002):
    if f(k, A):
        print(k)
        exit()
print(-1)
