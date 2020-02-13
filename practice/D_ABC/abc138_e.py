# https://atcoder.jp/contests/abc138/tasks/abc138_e

# s'は無限回繰り返されているとみなす
# sの中に含まれていない文字でtが構成されていたら速攻で-1

# 1周あたりの消化量はsの文字とその出現回数
# 順番が絡むのは最初の一回だけ(部分文字列として消化できない文字の可能性がある)
import sys
read = sys.stdin.readline

s = read()[:-1]
t = read()[:-1]

'''
1234567    ...                      33
contest contest contest contest contest contest
     s      e     nte     n     c   e
'''
