# https://atcoder.jp/contests/agc032/tasks/agc032_a
# 条件を満たす中で数字の大きい方からgreedyに抜けばよい
# もしルールに従って数字が挿入されているならばそのような数字が必ず一つは存在する


N = int(input())
*B, = map(int, input().split())

ans = []
for _ in range(N):
    for i in range(len(B) - 1, -1, -1):
        if i + 1 == B[i]:
            ans.append(B[i])
            del B[i]
            break
    else:
        print(-1)  # 見つからなかったら
        exit()
print(*reversed(ans), sep='\n')
