S = input()

keyword = 'keyence'
if (S[:7] == keyword) or (S[-7:] == keyword):
    print('YES')
    exit()

f, b = 0, 0
# forward
for i, (key, s) in enumerate(zip(keyword, S)):
    if key != s:
        f = i
        break
    else:
        # keyword = keyword[1:]
        pass
# print(keyword)
# backward
for i, (key, s) in enumerate(zip(keyword[::-1], S[::-1])):
    # print(key, s)
    if key != s:
        b = i
        break
    else:
        keyword = keyword[1:]

# print(f, b)
if f + b > 6:
    print('YES')
else:
    print("NO")
