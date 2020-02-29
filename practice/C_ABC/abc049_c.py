# https://atcoder.jp/contests/abc049/tasks/arc065_a
# なんかこの問題知ってる気がする
# 前方向から貪欲にマッチしようとすると,dreamer なのか dream eraser なのかわかりにくい
# しかし後ろ三文字を見ると、eam,mer,ase,serと一意に決定できる。よって逆方向から貪欲にマッチングさせると即座に構成可能か判別できる

S = input()

i = len(S)
while i - 3 > 0:
    tri = S[i - 3:i]
    if tri == 'eam':
        is_ok = S[i - 5:i] == 'dream'
        i -= 5
    elif tri == 'ase':
        is_ok = S[i - 5:i] == 'erase'
        i -= 5
    elif tri == 'mer':
        is_ok = S[i - 7:i] == 'dreamer'
        i -= 7
    elif tri == 'ser':
        is_ok = S[i - 6:i] == 'eraser'
        i -= 6
    else:
        print('NO')
        exit()

    if not is_ok:
        print('NO')
        exit()


print('YES')
