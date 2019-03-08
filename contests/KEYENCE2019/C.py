N = int(input())
A = list(map(int, input().split()))
B = list(map(int, input().split()))

if sum(A) < sum(B):
    print(-1)
    exit()

isgoukaku = True
fusokus = []
yobun = []

for a, b in zip(A, B):
    if a < b:
        # 不足点系列
        fusokus.append(b - a)
        isgoukaku = False
    else:
        # 余分な点系列
        yobun.append(a - b)
# print(fusokus, yobun)


if isgoukaku:
    print(0)
    exit()

fusoku = sum(fusokus)
yobun.sort(reverse=True)

henkou = len(fusokus)
# print(henkou)
tmp = 0
for y in yobun:
    # print(tmp, fusoku)
    if tmp >= fusoku:
        break
    tmp += y
    henkou += 1
    # print(henkou)

print(henkou)
