from operator import itemgetter
n = 5
s = [1, 2, 4, 6, 8]
t = [3, 5, 7, 9, 10]

st = [(ss, tt) for ss, tt in zip(s, t)]
# 選べる仕事の中で一番早く終る仕事を選ぶ→終わる順にソートしておくと便利
st.sort(key=itemgetter(1))
pre_t = 0  # 前の終了時刻
ans = 0
for s, t in st:
    if s <= pre_t:
        continue  # 前の終了時より前なので始められない
    ans += 1
    pre_t = t
print(ans)
