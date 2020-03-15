# https://atcoder.jp/contests/abc001/tasks/abc001_3
# 水パフォなのに虚無か？


def deg_to_dir(deg):
    deg *= 10
    a = 1125
    sa = 2250
    if a <= deg < a + sa:
        return 'NNE'
    elif a + sa <= deg < a + sa * 2:
        return 'NE'
    elif a + sa * 2 <= deg < a + sa * 3:
        return 'ENE'
    elif a + sa * 3 <= deg < a + sa * 4:
        return 'E'
    elif a + sa * 4 <= deg < a + sa * 5:
        return 'ESE'
    elif a + sa * 5 <= deg < a + sa * 6:
        return 'SE'
    elif a + sa * 6 <= deg < a + sa * 7:
        return 'SSE'
    elif a + sa * 7 <= deg < a + sa * 8:
        return 'S'
    elif a + sa * 8 <= deg < a + sa * 9:
        return 'SSW'
    elif a + sa * 9 <= deg < a + sa * 10:
        return 'SW'
    elif a + sa * 10 <= deg < a + sa * 11:
        return 'WSW'
    elif a + sa * 11 <= deg < a + sa * 12:
        return 'W'
    elif a + sa * 12 <= deg < a + sa * 13:
        return 'WNW'
    elif a + sa * 13 <= deg < a + sa * 14:
        return 'NW'
    elif a + sa * 14 <= deg < a + sa * 15:
        return 'NNW'
    else:
        return 'N'


from decimal import Decimal, ROUND_HALF_UP


def dis_to_W(dis):
    dis = Decimal(dis)
    dis /= 6
    dis = dis.quantize(Decimal('1'), rounding=ROUND_HALF_UP)
    if 0 <= dis <= 2:
        return 0
    elif 3 <= dis <= 15:
        return 1
    elif 16 <= dis <= 33:
        return 2
    elif 34 <= dis <= 54:
        return 3
    elif 55 <= dis <= 79:
        return 4
    elif 80 <= dis <= 107:
        return 5
    elif 108 <= dis <= 138:
        return 6
    elif 139 <= dis <= 171:
        return 7
    elif 172 <= dis <= 207:
        return 8
    elif 208 <= dis <= 244:
        return 9
    elif 245 <= dis <= 284:
        return 10
    elif 285 <= dis <= 326:
        return 11
    else:
        return 12


a, b = map(int, input().split())
dir = deg_to_dir(a)
W = dis_to_W(b)
print(dir if W != 0 else 'C', W)
