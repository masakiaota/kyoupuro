# https://atcoder.jp/contests/abc139/tasks/abc139_d
# 数字iに対して余りが最大となるのは,数字iがそのままあまりとして出てくる場合
# よって数字iをi+1の数字で割ってあげるのが一番余りが大きくなる
# 最後だけ1で割らないと行けなくて0になってしまうことに注意

N = int(input())
print((N * (N - 1)) // 2)
