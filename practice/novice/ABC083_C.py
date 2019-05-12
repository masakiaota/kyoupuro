# 問題
# https://atcoder.jp/contests/abc083/tasks/arc088_a

# 方針 greedy
# Xから初めて、2倍2倍していってYに達する手前で止めればよい
# この倍々操作の回数が数列の長さだが、これは簡単に数値計算で求まる
# つまり log_2(X/Y) を整数に丸め込んだやつ
# log2の丸め込み誤差に注意


from math import log2
X, Y = list(map(int, input().split()))

# print(int(log2(Y / X)+1)) #いや、あってるが…
print(len(bin(Y // X))-2)
