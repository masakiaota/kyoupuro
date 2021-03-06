import sys
read = sys.stdin.readline


# 下の桁から貪欲に見ていくと解ける
# 1の位が4以下ならそのまま紙幣で支払うのが得→自身をそのまま答えに採用 (枚数として加算)
# 6以上ならばお釣り狙いのほうが得→自身は0にセットして、上位桁を+1する (枚数として10-自身を加算)
# 5ちょうどのときは、お釣りもそのまま払っても枚数は変わらない。だけど555みたいに、上位桁に+1すると得になる可能性がある。
# 555→15枚, 1000-445→15枚
# つまり5が出たときは更に上位の桁を見て、4以下ならそのまま払うほうが得,5以上なら桁上りのほうが得


N = '00' + read()[:-1]

N = N[::-1]
ans = 0
i = 0

lo = int(N[i])
while i < len(N) - 1:
    hi = int(N[i + 1])
    if lo == 5:  # 5のときは上の桁の大きさによってお釣りが得になるかそのまま払うのが良くなるか変化する
        ans += lo
        if hi < 5:
            pass
        else:
            hi += 1
    elif lo < 5:
        # loが5以外のとき (お釣りが良いか、そのまま払うのが良いか考えるだけ)
        ans += lo
    else:
        ans += 10 - lo
        hi += 1  # 上の桁が繰り上がる
    lo = hi  # 桁を一つずらす
    i += 1


print(ans)
