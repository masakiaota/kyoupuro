using namespace std;
typedef long long ll;
#include<bits/stdc++.h>
#define REP(i,n) for(ll i=0;i<ll(n);i++)
#define REPD(i,n) for(ll i=n-1;i>=0;i--)
#define FOR(i,a,b) for(ll i=a;i<=ll(b);i++)
#define FORD(i,a,b) for(ll i=a;i>=ll(b);i--)
#define FORA(i,I) for(const auto& i:I)
//xにはvectorなどのコンテナ
#define ALL(x) x.begin(),x.end() 
#define SIZE(x) ll(x.size()) 
//定数
#define INF 1000000000000 //10^12:∞
#define MOD 1000000007 //10^9+7:合同式の法
#define MAXR 100000 //10^5:配列の最大のrange
//略記
#define PB push_back //挿入
#define MP make_pair //pairのコンストラクタ
#define F first //pairの一つ目の要素
#define S second //pairの二つ目の要素

struct Mo
{
  vector< int > left, right, order;
  vector< bool > v;
  int width;
  int nl, nr, ptr;

  Mo(int n) : width((int) sqrt(n)), nl(0), nr(0), ptr(0), v(n) {}

  void insert(int l, int r) /* [l, r) */
  {
    left.push_back(l);
    right.push_back(r);
  }

  /* ソート */
  void build()
  {
    order.resize(left.size());
    iota(begin(order), end(order), 0);
    sort(begin(order), end(order), [&](int a, int b)
    {
      if(left[a] / width != left[b] / width) return left[a] < left[b];
      return right[a] < right[b];
    });
  }

  /* クエリを 1 つぶんすすめて, クエリのidを返す */
  int process()
  {
    if(ptr == order.size()) return (-1);
    const auto id = order[ptr];
    while(nl > left[id]) distribute(--nl);
    while(nr < right[id]) distribute(nr++);
    while(nl < left[id]) distribute(nl++);
    while(nr > right[id]) distribute(--nr);
    return (order[ptr++]);
  }

  inline void distribute(int idx)
  {
    v[idx].flip();
    if(v[idx]) add(idx);
    else del(idx);
  }

  void add(int idx);

  void del(int idx);
};

int N, A[500010], Q;
int ans[500010];
int cnt[1000001], sum;

void Mo::add(int idx)
{
  if(cnt[A[idx]]++ == 0) ++sum;
}

void Mo::del(int idx)
{
  if(--cnt[A[idx]] == 0) --sum;
}

int main()
{
  scanf("%d", &N);
  scanf("%d", &Q);
  for(int i = 0; i < N; i++) {
    scanf("%d", &A[i]);
  }
  Mo mo(N);
  for(int i = 0; i < Q; i++) {
    int a, b;
    scanf("%d %d", &a, &b);
    mo.insert(--a, b);
  }
  mo.build();
  for(int i = 0; i < Q; i++) {
    ans[mo.process()] = sum;
  }
  for(int i = 0; i < Q; i++) {
    printf("%d\n", ans[i]);
  }
}