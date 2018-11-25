N = int(input())
import heapq as h
A = list(map(int, input().split()))


# use 0 based index
sum_red, sum_blue = [], []

head = A[:N]
tail = [-x for x in A[-N:]]
h.heapify(head)
h.heapify(tail)
sum_red.append(sum(head))
sum_blue.append(-sum(tail))
# keypoint: いちいちsumすると線形時間かかるのでsum_hoge[-1]でアクセスする
for k in range(N, 2*N):
    num_sub = h.heappushpop(head, A[k])
    sum_red.append(sum_red[-1]-num_sub+A[k])
    num_sub = -h.heappushpop(tail, -A[-k-1])
    sum_blue.append(sum_blue[-1]-num_sub+A[-k-1])


print(max([r-b for r, b in zip(sum_red, sum_blue[::-1])]))
