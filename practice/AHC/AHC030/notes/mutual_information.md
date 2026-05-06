# 相互情報量による占い集合の評価

## 目的

占い集合 $S$ を、真の油田配置 $\theta^\star$ の不確実性を効率よく下げるように選びたい。

そのため、占い結果を確率変数 $Y$ として、

$$
I(\theta^\star;Y\mid \mathcal{D}_t,S)
$$

を用いる。

これは、

「これまでの観測履歴 $\mathcal{D}_t$ がある状態で、次に占う集合 $S$ を固定したとき、その占い結果 $Y$ が真の油田配置 $\theta^\star$ について平均的にどれだけ情報を与えるか」

を表す。

占いコストは

$$
\mathrm{cost}(S)=\frac{1}{\sqrt{|S|}}
$$

なので、実装ではコストあたりの情報量として

$$
\frac{I(\theta^\star;Y\mid \mathcal{D}_t,S)}{\mathrm{cost}(S)}
=
\sqrt{|S|}I(\theta^\star;Y\mid \mathcal{D}_t,S)
$$

を評価した。

## 事後分布 $p_t(\theta)$

候補 $\theta\in\Theta$ の現在の事後確率を

$$
p_t(\theta)
=
P(\theta^\star=\theta\mid\mathcal{D}_t)
$$

とおく。

観測履歴

$$
\mathcal{D}_t=\{(S_1,y_1),\ldots,(S_t,y_t)\}
$$

に対する対数尤度を

$$
\ell_t(\theta)
=
\sum_{\tau=1}^{t}
\log P(Y=y_\tau\mid\theta,S_\tau)
$$

とすると、一様事前分布のもとで

$$
p_t(\theta)
=
\frac{\exp(\ell_t(\theta))}
{\sum_{\theta'\in\Theta}\exp(\ell_t(\theta'))}
$$

である。

数値計算では、最大値

$$
L=\max_{\theta\in\Theta}\ell_t(\theta)
$$

を引いて、

$$
p_t(\theta)
=
\frac{\exp(\ell_t(\theta)-L)}
{\sum_{\theta'\in\Theta}\exp(\ell_t(\theta')-L)}
$$

と計算する。

## 素朴な相互情報量の計算

$p_t(\theta)$ と $P(Y=y\mid\theta,S)$ が計算できるとする。

まず、占い結果 $Y$ の予測分布を作る。

$$
q(y)
=
P(Y=y\mid\mathcal{D}_t,S)
=
\sum_{\theta\in\Theta}
p_t(\theta)P(Y=y\mid\theta,S)
$$

このとき、相互情報量は

$$
I(\theta^\star;Y\mid\mathcal{D}_t,S)
=
\sum_{\theta\in\Theta}
\sum_y
p_t(\theta)P(Y=y\mid\theta,S)
\log
\frac{
P(Y=y\mid\theta,S)
}{
q(y)
}
$$

で計算できる。

直感的には、候補 $\theta$ ごとに $P(Y=y\mid\theta,S)$ が大きく異なるほど、返ってきた $Y$ から $\theta^\star$ を区別しやすい。そのため相互情報量は大きくなる。

逆に、全ての候補 $\theta$ で $P(Y=y\mid\theta,S)$ がほぼ同じなら、$Y$ を観測しても $\theta^\star$ についてほとんど分からないため、

$$
I(\theta^\star;Y\mid\mathcal{D}_t,S)\approx 0
$$

となる。

## $v_\theta(S)$ による集約

実装では、候補 $\theta$ そのものではなく、

$$
v_\theta(S)
$$

ごとに集約して計算した。

理由は、占い応答の分布

$$
P(Y=y\mid\theta,S)
$$

は $\theta$ そのものではなく、候補 $\theta$ のもとでの埋蔵量

$$
v_\theta(S)
$$

だけで決まるからである。

そこで、

$$
w_v(S)
=
\sum_{\theta:\ v_\theta(S)=v}p_t(\theta)
$$

を作る。

すると、予測分布は

$$
P(Y=y\mid\mathcal{D}_t,S)
=
\sum_v w_v(S)P(Y=y\mid v,S)
$$

と書ける。

また、相互情報量はエントロピー差として

$$
I(\theta^\star;Y\mid\mathcal{D}_t,S)
=
H(Y\mid\mathcal{D}_t,S)
-
\sum_v w_v(S)H(Y\mid v,S)
$$

と計算できる。

ここで、

$$
H(Y\mid\mathcal{D}_t,S)
=
-
\sum_y
P(Y=y\mid\mathcal{D}_t,S)
\log P(Y=y\mid\mathcal{D}_t,S)
$$

である。

## 実装上の解釈

ある $S$ を評価するとき、実装では次を行う。

1. 各候補 $\theta$ について $v_\theta(S)$ を計算する。
2. 現在の事後分布 $p_t(\theta)$ を $v_\theta(S)$ ごとに集約し、$w_v(S)$ を作る。
3. $w_v(S)$ と $P(Y=y\mid v,S)$ から $P(Y=y\mid\mathcal{D}_t,S)$ を作る。
4. エントロピー差から $I(\theta^\star;Y\mid\mathcal{D}_t,S)$ を計算する。
5. 最終的に $\sqrt{|S|}I(\theta^\star;Y\mid\mathcal{D}_t,S)$ を $S$ の評価値とする。

## 今回の実験での整理

相互情報量を用いると、ランダムな占い集合よりも情報の取り方を改善できる場合がある。

特に $M=2$ のように候補数が小さいケースでは、事後分布を明示的に扱う計算が比較的軽いため、相互情報量に基づく占い集合最適化が有効だった。

一方で $M=3$ では、相互情報量評価の計算量が重くなり、うまくいくケースと失敗するケースが分かれた。占いの質は上がる可能性があるが、計算時間を使いすぎると late answer に入りやすくなり、失敗が増える。

したがって、現時点のまとめは次の通りである。

- $M=2$: 相互情報量ベースの占い集合選択は有効。
- $M=3$: 有効な場合もあるが、計算量とのトレードオフが不安定。
- $M\ge 4$: 全探索ベースの事後分布管理自体が厳しく、別方針が必要。
