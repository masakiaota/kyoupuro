# 愚直に操作をじっそうするだけじゃないんか?→Nの制約がめっちゃデカかったわ
# 操作は5*6回ごとにループする

N = int(input()) % 30
cards = list(range(1, 7))
for i in range(N):
    a = i % 5
    b = a + 1
    cards[a], cards[b] = cards[b], cards[a]
print(*cards, sep='')
