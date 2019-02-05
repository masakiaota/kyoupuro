# dn 解説の要点は2つ
# 1,各桁のbitは独立に決定できる(一番大事)(解説後半)
# 2,K以下で探すように成約を入れる（解説前半）


N, K = list(map(int, input().split()))
A = list(map(int, input().split()))


ans = 0

for i in range(40, -1, -1):
    # print(ans)
    # 桁の大きい方から探索していく.
    power = 1 << i  # i個桁をシフトさせることでi桁目について考えることができる。
    # まずは1の処理からしていこう
    # i番目のビットに1はいくつあるか
    bit_count_of_1 = sum(1 for a in A if a & power != 0)  # この書き方は頭いい

    # 寄与の分岐
    # 注意すべきはpower>Kのときで、ビットは必ず0なので寄与はその時点で決まる。
    if power > K:
        # print(i)
        ans += power * bit_count_of_1
        # print(ans)
    elif bit_count_of_1 > N - bit_count_of_1:
        # その桁で1のほうが多い場合は最適なXは0を与えたほうがいい。0を与えるのはつまり その桁の大きさ×1の個数
        ans += power * bit_count_of_1
    else:
        ans += power * (N - bit_count_of_1)
        K -= power  # なにげに一番むずかしいポイントはここだった
        #→power==Kかつ ans X_kが1を与えるほうが良いとき、一つ下の桁も問題の条件を満たしているかはわからない。そこでこの演算が必要
        # イメージとしては最上位のビットを削除している。
        # その結果次のiterで一つ下の桁が問題条件（K以下）かを判断することができる。


print(ans)
