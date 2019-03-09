def readln():
    return list(map(int, input().split()))


A, B = readln()


A_isodd = []  # 奇数の桁は1
B_isodd = []  # 奇数の桁は1
# 1の偶奇
tmp = (A) % 4
if (tmp == 2)or(tmp == 3):
    A_isodd.append(1)
else:
    A_isodd.append(0)
tmp = (B+1) % 4
if (tmp == 2)or(tmp == 3):
    B_isodd.append(1)
else:
    B_isodd.append(0)


for i in range(1, 40):  # 各桁につい
    tmp = (A) % 2 ** (i + 1)
    if (tmp > 2 ** (i)) and (tmp % 2 == 1):
        A_isodd.append(1)
    else:
        A_isodd.append(0)
    tmp = (B + 1) % 2 ** (i + 1)
    if (tmp > 2 ** (i)) and (tmp % 2 == 1):
        B_isodd.append(1)
    else:
        B_isodd.append(0)

# print(A_isodd)
# print(B_isodd)

ans = []
for a, b in zip(A_isodd, B_isodd):
    if a == b:
        ans.append(0)
    else:
        ans.append(1)

ans_print = 0
for i, a in enumerate(ans):
    ans_print += 2 ** i if a == 1 else 0

print(ans_print)
