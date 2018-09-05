input()
nums = list(map(int, input().split()))
cnt = 0
while True:
    isodd = [n % 2 for n in nums]
    if 1 in isodd:
        print(cnt)
        break
    nums = [int(n / 2) for n in nums]
    cnt += 1
