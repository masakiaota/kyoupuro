# https://atcoder.jp/contests/abc114/tasks/abc114_c

# 7,5,3がそれぞれ1回以上現れる数
# 3進数を自分で作ってしまうのは？→3,5,7の順にiterateするのは十分間に合う
# 10進数で10**9は3進数だけ考慮するなら10**4.5ぐらいですむ


from itertools import product
N = int(input())
number = ['0'] * 9
nums = ['0', '3', '5', '7']
arg = [nums] * 9


ans = 0
for iters in product(*arg):  # 頭がわるいので
    tmp = int(''.join(iters))
    if tmp > N:
        break
    tmp = str(tmp)
    if tmp.count('0') > 0 or tmp.count('3') == 0 or tmp.count('5') == 0 or tmp.count('7') == 0:
        continue
    else:
        ans += 1


print(ans)
