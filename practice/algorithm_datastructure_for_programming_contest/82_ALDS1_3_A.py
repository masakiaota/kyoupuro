# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/all/ALDS1_3_A
# 逆オペランド表記の理解とスタックの練習

# INPUT = [int(x) if x.isdigit() else x for x in input().split()]
INPUT = list(input().split())
stack = []  # listを用いてスタックを実装する

for x in INPUT:
    if x.isdigit():
        stack.append(x)
    else:
        # 引き算の場合は順序が考慮されるので注意
        second_item = stack.pop()
        first_item = stack.pop()
        tmp = eval(first_item + x + second_item)
        stack.append(str(tmp))

print(int(stack.pop()))
