N, A, B = list(map(int, input().split()))

suma = 0
for n in range(1, N + 1):
    n_keta = []
    tmp = n
    while True:
        n_keta.append(tmp % 10)
        tmp //= 10
        if tmp == 0:
            break
    if A <= sum(n_keta) <= B:
        suma += n
print(suma)
