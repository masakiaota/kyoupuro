def readln():
    return list(map(int, input().split()))


A, B = readln()

ans = []
for i in range(40):
    power = 1 << i  # i桁目について考える
    bit_count_of_1 = sum(1 for a in range(A, B + 1) if a & power != 0)
    if bit_count_of_1 % 2 == 1:
        ans.append(1)
    else:
        ans.append(0)

ans_print = 0
for i, a in enumerate(ans):
    ans_print += 2 ** i if a == 1 else 0

print(ans_print)
