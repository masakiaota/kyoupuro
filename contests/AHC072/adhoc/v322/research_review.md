# v113に使える技法の調査とv322の設計

## 調査の結論

今回の実装対象は、**完成済みの操作列から少数の操作を削除し、その影響だけを後続へ伝える局所書換え**とした。v113のNN、初期4候補、二候補育成、既存の除去と再挿入は残す。新方式の最終評価は[実験ノート](../../notes/experiments/v322.md)に記録し、本調査の有望性判断と区別する。

対象ソースはGitの`7e5c9ba03eadf0a7bb143a476b9f878ecc341cbb`にある`v113_integrated_nn_lns.cpp`。重み込みSHA256は`037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258`である。validation1の入力、出力、評価CSVは調査用に取り込まず、新規生成128件とin100だけを今回の完成版評価に使う。

文献は論文の著者公開ページ、会議公式ページ、出版社ページを優先した。下表は関連技法の適用条件を整理したもので、各論文の全実験を再現したという意味ではない。PbRは本文の3.1.3〜3.1.5を確認し、他の多くは著者または出版社の抄録と実装公開情報までを確認した。PbRのPDFは本文抽出を確認できたが、Webの画像レンダリングはエラーとなり、図の細部を根拠にしていない。今回の検索では、このコンペの上位者による具体的な公開解法を十分に特定できなかった。

## 1. 問題と既存コードから確認できること

**この問題は、独立した個体の移動距離を足す問題ではない。** 1操作で複数のスライムが移動し、下段が支持力になる。着地で順番が反転し、出発点と着地点の帰巣が将来の足場を消す。目的関数は操作数Tと残存罰則100000Eである。したがって、MAPFの衝突回避やVRPの顧客挿入の改善率を、そのまま手数改善として見積もれない。根拠は`problem_description.txt`のSomersault Jump、Returning Home、Scoringである。

**v113には既に多くの定番手法が入っている。** NNによる構築、回転反転による候補生成、候補の短時間育成、複数の除去近傍、予定に沿った動的計画法、同色で同じ盤面になる挿入位置の統合、転送圧縮、短区間の共同探索、履歴集計の共有がある。新しい名前の手法を足す前に、同じ働きをする部品がないかを照合した。根拠は固定したv113ソースの`nn::search`、`grow_initial_solutions`、`TemporalLNS`、`close_color_gaps`、`SearchReductions`である。

**小さな変更でも、元の盤面へ戻るまで長い期間が必要な例がある。** backlogのO75では、過去の長時間探索の更新について、64操作では閉じない短縮の中にも、変更操作や接触マスが少ないものが記録されている。今回、その元の全更新列を再集計したわけではない。保存観察を設計の動機として使い、効果は新しい入力で測る。

## 2. 文献をv113へ対応付ける

「接続案」と「判断」は、本問題とコードを踏まえた今回の推論である。文献自体がAHC072での有効性を示したものではない。

| 技法と一次資料 | 文献が扱う要点 | v113への接続案と今回の判断 |
|---|---|---|
| Planning by Rewriting [1] | 既存の計画に書換え規則を適用し、変更が壊す条件を回復して有効な計画へ戻す。 | 個体の全履歴を消す代わりに操作を消し、盤面差だけを直す。今回のv322として実装する。宣言的規則言語やrPOPをそのまま移植したわけではない。 |
| MAPF-LNS [2] | 完成した経路集合から一部のエージェントを再計画する。 | 既存のTemporalLNSと方向が近い。衝突を避ければよい問題と違い、ここでは他個体が有益な足場でもある。単純に除去対象を増やすだけでは十分でない。 |
| MAPF-LNS2 [3] | 衝突を含む初期経路を、部分集合の再計画で修復する。 | 内部の未解決条件を明示する発想は使える。ただし未修復列は出力できない。今回も全盤面へ接続できた完成列だけを返す。 |
| SIPP [4] | 多数の時刻を、同じ安全性を持つ時間区間へまとめる。 | v113は既に盤面イベントで変わる2マスと周辺を開き直す。さらに区間化するには色順、容量、支持の変化を区別する必要があり、時間の次元を消すだけの移植はしない。 |
| SISR [5] | 隣接する経路部分の除去と、候補を一部省く再挿入を組み合わせる。 | 輸送上の近さを除去単位に使う案は残る。ただし個体の初期距離だけでは支持期間を表せない。今回の新近傍とは分ける。 |
| ALNS [6]、BALANCE [7]、ADDRESS [8] | 近傍の種類や対象規模、対象個体の選び方を適応させる。 | v120の適応配分は既に不採用である。別の報酬設計を伴わずバンディットへ交換するだけの再試行は行わない。 |
| Learning to Delegate [9]、MAPF-ML-LNS [10] | 学習で改善すべき部分問題を選び、既存の探索器に解かせる。 | 全ゲームの方策再学習より接続しやすい。ただし現行の候補順位NNと用途が重なる。新しい部分問題の生成能力を確認してから、その成功条件を学ぶ順序を優先する。 |
| Policy-Guided Heuristic Search [11] | 方策とヒューリスティックを探索順位に組み合わせる。 | 方策の高確率は短い操作列の保証ではない。推論費もあり、良い下界なしに大きな全盤面探索へ置き換えるのは今回見送る。 |
| Simulation-Guided Beam Search [12] | NNとロールアウトで有望なビーム候補を選ぶ。 | v313の別操作ロールアウトやv124以降の計画選別と重なる部分がある。NN構築の改善だけでなく、その分減るLNS時間を含めた利益を先に問う必要がある。 |
| Efficient Active Search [13] | テスト時にモデルの一部を更新して探索を誘導する。 | 既存重みを活用する候補だが、2秒CPU内での逆伝播と試行費用が未確認である。今回の提出へ急に入れず、追加計算を使う別実験に分離する。 |
| QAPのejection chain [14] | 複数の関連変更をつなげて大きな近傍を扱う。 | 一つの変更の影響を次の変更で吸収する発想は近い。QAPの交換費用式はこのゲームに流用できないため、v322は色盤面の実再生で接続を検査する。 |

## 3. 今回繰り返さなかったこと

取得したbacklogと関連実験ノートには、学習器の損失改善や局所の短縮が完成版へ残らない例がある。これらを手法一般の否定にはしないが、同じ構成を理由なく再試行しない。

| 既存の試み | 確認した限界 | 今回の対応 |
|---|---|---|
| v124〜v129の初期計画選別 | 事後的な最良計画には余地があっても、早期の特徴から未知入力へ利益を移せなかった。 | 同じ早期特徴の選択器を作り直さない。 |
| v130の最良記録を基準にした受理 | 一時悪化を多く受け入れても完成手数が減らなかった。 | 受理式と温度を変更しない。 |
| v131の同費用の再挿入二経路 | 追加枝が短い完成列を作る例はあるが、再挿入費が増えて最終比較では悪化した。 | 各個体に重い別経路を残す代わりに、操作列の差分が小さい範囲を直接探す。 |
| v064/v065の区間再挿入 | 任意終点の準備や探索に費用がかかり、完成版で利益を得られなかった。 | 対象個体の再挿入をやめ、既存操作の変更と省略だけに絞る。 |
| v307の永久同色結合 | 状態は減っても、必要な分離や支持の選択肢を失った。 | 同色のIDを区別しないことと、同色を分割禁止にすることを分ける。分割は禁止しない。 |

この表は各版の保存記録の要約であり、今回の環境で各版を再評価した比較ではない。validation1のケース別成績を分析したものでもない。

## 4. v322の状態表現と探索

ある元の操作の直前を開始点にし、その操作を1回省く。以後は、同じ時刻の元の盤面に対して色順が違うマスだけを保存する。差分は最大4マス、各マスの塔は32bitの色列である。個体番号の順列は状態へ入れない。空塔の差分も保存する。

次の元操作の出発点と着地点がどちらも差分外なら、その操作はそのまま進める。差分へ触る操作については、元操作の再生、同じ着地点へ運ぶ匹数の変更、別の差分マスから同じ着地点への直接移動、操作の省略を候補にする。支持距離、壁、帰巣前の容量、反転、両端の帰巣は各候補で実際に判定する。

状態を統合するのは、元列の時刻、差分の色盤面、編集済み数、省略済み数が同じ場合だけである。残りの編集予算が異なる状態をまとめると、後で使える操作が変わるためである。ハッシュ表を毎回作らず、最大8候補の小さな配列で比較する。

探索は元列256操作まで、幅8、編集12回まで、省略3回までとする。追加操作はない。未一致の色数とマス数などをビーム順位に使うが、最短費用の下界とはみなさない。この制限により見つからない短縮がある。

全差分が消えた時点で、元列の残りを接続する。出力前に完全な盤面再生で接続境界の一致と全帰巣を検査する。残存ゼロの元計画から、合法な変更を行い、接続境界で全盤面が等しいなら、元の後半の各操作も同じ状態へ作用する。この帰納的な理由で完成列の合法性を保つ。ビーム探索があらゆる短縮を発見できるという保証ではない。

追加試行は29反復ごとの枠を使い、各LNSの使用時間の6%を目安に抑える。1呼出しの期限は総予算の0.00075倍で、区間の終了と全体の終了にも制限される。元のNNと他のLNS機構、受理、育成、時計は固定する。時間依存探索なので、新近傍から直接縮めた手数と、親に対する完成版の差を区別して報告する。

## 5. この設計でまだ扱えない短縮

新しい中継点へまず1手置いてから回収する計画、5マス以上の差分を同時に必要とする計画、13操作以上の編集や256操作以上の期間を必要とする計画は対象外である。着地点の候補は元操作の着地点に固定しているため、任意の二端点の共同移動でもない。

これらは今回の結果から自動的に広げない。追加するほど状態数と処理費が増え、これまでの局所改善と最終悪化を再現し得るためである。次の実験で調べるなら、保存された取りこぼしがどの制限に阻まれたかを先に特定する。今回の出力を使ってパラメータを後から選び直すことはしない。

## 一次資料

1. Ambite, J. L.; Knoblock, C. A. **Planning by Rewriting**. JAIR 15, 207–261, 2001. [著者公開版](https://arxiv.org/abs/1106.0250)、[DOI](https://doi.org/10.1613/jair.754)。本文3.1.3の書換えと条件修復、3.1.4の探索費、3.1.5の規則分類を確認した。
2. Li et al. **Anytime Multi-Agent Path Finding via Large Neighborhood Search**. IJCAI 2021. [会議公式](https://www.ijcai.org/proceedings/2021/568)。
3. Li et al. **MAPF-LNS2: Fast Repairing for Multi-Agent Path Finding via Large Neighborhood Search**. AAAI 2022. [DOI](https://doi.org/10.1609/aaai.v36i9.21266)。
4. Phillips, M.; Likhachev, M. **SIPP: Safe Interval Path Planning for Dynamic Environments**. ICRA 2011. [著者所属機関](https://publications.ri.cmu.edu/sipp-safe-interval-path-planning-for-dynamic-environments)。
5. Christiaens, J.; Vanden Berghe, G. **Slack Induction by String Removals for Vehicle Routing Problems**. Transportation Science 54(2), 417–433, 2020. [出版社](https://pubsonline.informs.org/doi/abs/10.1287/trsc.2019.0914)。
6. Ropke, S.; Pisinger, D. **An Adaptive Large Neighborhood Search Heuristic for the Pickup and Delivery Problem with Time Windows**. Transportation Science 40(4), 455–472, 2006. [DOI](https://doi.org/10.1287/trsc.1050.0135)。
7. Phan et al. **Adaptive Anytime Multi-Agent Path Finding Using Bandit-Based Large Neighborhood Search**. [著者公開版](https://arxiv.org/abs/2312.16767)。
8. Phan et al. **Anytime Multi-Agent Path Finding Using an Adaptive Delay-Based Heuristic**. [著者公開版](https://arxiv.org/abs/2408.02960)。
9. Li, S.; Yan, Z.; Wu, C. **Learning to Delegate for Large-scale Vehicle Routing**. NeurIPS 2021. [著者公開版](https://arxiv.org/abs/2107.04139)、[所属研究機関](https://mitibm.mit.edu/research/blog/learning-to-delegate-for-large-scale-vehicle-routing/)。
10. Huang et al. **Anytime Multi-Agent Path Finding via Machine Learning-Guided Large Neighborhood Search**. AAAI 2022. [DOI](https://doi.org/10.1609/aaai.v36i9.21168)。
11. Orseau, L.; Lelis, L. H. S. **Policy-Guided Heuristic Search with Guarantees**. AAAI 2021. [会議公式](https://ojs.aaai.org/index.php/AAAI/article/view/17469)。
12. Choo et al. **Simulation-guided Beam Search for Neural Combinatorial Optimization**. NeurIPS 2022. [著者公開版](https://arxiv.org/abs/2207.06190)。
13. Hottung, A.; Kwon, Y.-D.; Tierney, K. **Efficient Active Search for Combinatorial Optimization Problems**. [著者公開版](https://arxiv.org/abs/2106.05126)。
14. Rego et al. **An ejection chain algorithm for the quadratic assignment problem**. Networks, 2010. [出版社](https://onlinelibrary.wiley.com/doi/10.1002/net.20360)。

公表年の古い文献も、現在の問題構造へ対応するなら比較対象とした。ここでの調査は理論と実装方針の選択に使い、他問題での倍率や最良成績をAHC072の期待改善量へ置き換えていない。
