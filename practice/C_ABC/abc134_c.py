# https://atcoder.jp/contests/abc134/tasks/abc134_c

N = int(input())
A = []
max1 = 0
max2 = 0

for _ in range(N):
    a = int(input())
    # if max1 <= a:
    #     max2 = max1
    #     max1 = a
    # elif max2 <= a:
    #     max2 = a
    A.append(a)

# for a in A:
#     print(max1 if a != max1 else max2)


# でも制約からこんなバグりやすいプログラム書かなくても
A_sorted = sorted(A)
max1 = A_sorted[-1]
max2 = A_sorted[-2]
for a in A:
    print(max1 if a != max1 else max2)
