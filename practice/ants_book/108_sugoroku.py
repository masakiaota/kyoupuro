# 蟻本ではかなり説明が端折られてるのでけんちょんさんのブログを読もう！
# https://qiita.com/drken/items/b97ff231e43bce50199a


def extgcd(a, b):
    '''ax + by = gcd(a,b) を満たすgcd(a,b),x,yを返す'''
    if b == 0:
        return a, 1, 0
    g, x, y = extgcd(b, a % b)
    return g, y, x - a // b * y


a = 4
b = 11

g, x, y = extgcd(a, b)


ans = []
if x >= 0:
    ans.append(x)
else:
    ans.append(0)

if y >= 0:
    ans.append(y)
else:
    ans.append(0)
if x < 0:
    ans.append(-x)
else:
    ans.append(0)
if y < 0:
    ans.append(-y)
else:
    ans.append(0)
print(*ans)

# いろいろ試す
g, x, y = extgcd(111, 30)
print(x, y)
g, x, y = extgcd(30, 111)
print(x, y)

g, x, y = extgcd(4, 2)
print(x, y)


def extgcd(a, b):  # 非再帰
    '''ax + by = gcd(a,b) を満たすgcd(a,b),x,yを返す'''
    x0, y0, x1, y1 = 1, 0, 0, 1
    while b != 0:  # 互除法の回数処理を行って
        q, a, b = a // b, b, a % b  # 上に伝播させていく
        x0, x1 = x1, x0 - q * x1
        y0, y1 = y1, y0 - q * y1
    return a, x0, y0


# いろいろ試す
g, x, y = extgcd(111, 30)
print(x, y)
g, x, y = extgcd(30, 111)
print(x, y)
g, x, y = extgcd(4, 2)
print(x, y)
