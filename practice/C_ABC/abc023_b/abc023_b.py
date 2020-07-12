# 文字はたかだか100文字、全通り作ってやってみれば良い
candi = ['b']
for i in range(60):
    if i % 3 == 0:
        candi.append('a' + candi[-1] + 'c')
    elif i % 3 == 1:
        candi.append('c' + candi[-1] + 'a')
    elif i % 3 == 2:
        candi.append('b' + candi[-1] + 'b')

input()
S = input()
candi.append(S)
ans = candi.index(S)
print(ans if ans != len(candi) - 1 else -1)
