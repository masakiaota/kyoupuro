# https://atcoder.jp/contests/arc046/tasks/arc046_b
# 最後は必ずA個かB個以内で勝利が確定する
# だめだ解説見る
#
# もしA==Bのとき→先にN%(1+A)==0の状態にできた方の勝ち
# 最初からN%(1+A)==0であればB(後攻)の勝ち。
# そうでなければ先行がN-a\in[1,A]を行うことで必ずN%(1+A)==0の状態にできるのでA(先行)の勝ち
# ∵最後に勝つ人はもう一方の行動にかかわらず、両者の合計をA+1にできるので、A+1の倍数というのが勝ちの確定する状態である

# A!=Bのとき→大きいほうが必ず勝利する
# ∵小さいほうがジリ貧になるまで1だけつかって大きいほうが最後に一気に0にできる

N, A, B = map(int, open(0).read().split())
F = 'Takahashi'
S = 'Aoki'
if A >= N:
    print(F)
elif A != B:
    print(F if A > B else S)
else:
    print(S if N % (A + 1) == 0 else F)
