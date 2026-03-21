//! 01_nearest_neighbor.rs
//! 今いる点から最も近いレストランに行くことを50回繰り返し、その後今いる点から最も近い目的地に行くことを50回繰り返す解法プログラム
//! 必要なクレートの導入には cargo add proconio@=0.4.5 rand@=0.8.5 rand_pcg@=0.3.1 を実行してください
use proconio::input;
use std::fmt::Display;

/// 2次元座標上の点を表す構造体
#[derive(Debug, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    /// コンストラクタ
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 2点間のマンハッタン距離を計算する
    fn dist(&self, other: Point) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.x, self.y)
    }
}

/// 入力データを表す構造体
struct Input {
    /// レストランの数 (=1000)
    order_count: usize,
    /// 選択する必要のある注文の数 (=50)
    pickup_count: usize,
    /// AtCoderオフィスの座標 (=(400, 400))
    office: Point,
    /// レストランの座標の配列
    restaurants: Vec<Point>,
    /// 目的地の座標の配列
    destinations: Vec<Point>,
}

impl Input {
    /// 入力データを読み込む
    fn read() -> Self {
        let order_count = 1000;
        let pickup_count = 50;
        let office = Point::new(400, 400);

        let mut restaurants = vec![];
        let mut destinations = vec![];

        for _ in 0..order_count {
            input! {
                a: i32,
                b: i32,
                c: i32,
                d: i32,
            }

            restaurants.push(Point::new(a, b));
            destinations.push(Point::new(c, d));
        }

        Self {
            order_count,
            pickup_count,
            office,
            restaurants,
            destinations,
        }
    }
}

/// 出力データを表す構造体
struct Output {
    /// 移動距離の合計
    dist_sum: i32,
    /// 選択した注文のリスト
    orders: Vec<usize>,
    /// 配達ルート
    route: Vec<Point>,
}

impl Output {
    /// コンストラクタ
    fn new(orders: Vec<usize>, route: Vec<Point>) -> Self {
        let mut dist_sum = 0;

        for i in 0..route.len() - 1 {
            dist_sum += route[i].dist(route[i + 1]);
        }

        Self {
            dist_sum,
            orders,
            route,
        }
    }

    /// 解を出力する
    fn print(&self) {
        // 選択した注文の集合を出力する
        print!("{}", self.orders.len());

        for &i in &self.orders {
            // 0-indexed -> 1-indexedに変更
            print!(" {}", i + 1);
        }

        println!();

        // 配達ルートを出力する
        print!("{}", self.route.len());

        for p in &self.route {
            print!(" {}", p);
        }

        println!();
    }
}

/// 問題を解く関数（この関数を実装していきます）
fn solve(input: &Input) -> Output {
    // 貪欲その1
    // 以下を順に実行するプログラム
    // 1.高橋君は最初オフィスから出発する
    // 2.訪問したレストランが50軒に達するまで、今いる場所から一番近いレストランに移動することを繰り返す
    // 3.受けた注文を捌ききるまで、今いる場所から一番近い配達先に移動することを繰り返す
    // 4.オフィスに帰る

    let mut orders = vec![]; // 注文の集合
    let mut route = vec![]; // 配達ルート

    // 1. オフィスからスタート
    route.push(input.office);
    let mut current_position = input.office; // 現在地
    let mut total_dist = 0; // 総移動距離

    // 2.訪問したレストランが50軒に達するまで、今いる場所から一番近いレストランに移動することを繰り返す

    // 同じレストランを2回訪れてはいけないので、訪問済みのレストランを記録する
    let mut visited_restaurant = vec![false; input.order_count];

    // pickup_count(=50)回ループ
    for i in 0..input.pickup_count {
        // レストランを全探索して、最も近いレストランを探す
        let mut nearest_restaurant = 0; // レストランの番号
        let mut min_dist = 1000000; // 最も近いレストランの距離

        for j in 0..input.order_count {
            // 【穴埋め】既に訪れていたらスキップ
            /* put your code here */

            // 【穴埋め】最短距離が更新されたら記録
            // 【ヒント】let distance = p0.dist(p1); と書くと、p0とp1のマンハッタン距離が計算できる
            // 【ヒント】nearest_restaurant, min_distの2つを更新する
            /* put your code here */
        }

        // 最も近いレストラン(nearest_restaurant)に移動する
        // 【穴埋め】現在位置を最も近いレストランの位置に更新
        /* put your code here */

        // 【穴埋め】注文の集合に選んだレストランを追加
        /* put your code here */

        // 【穴埋め】配達ルートに現在の位置を追加
        /* put your code here */

        // 【穴埋め】訪問済みレストランの配列にtrueをセット
        /* put your code here */

        // 総移動距離の更新
        total_dist += min_dist;

        // デバッグしやすいよう、標準エラー出力にレストランを出力
        // 標準エラー出力はデバッグに有効なので、AHCでは積極的に活用していきましょう
        let restaurant_pos = input.restaurants[nearest_restaurant];
        eprintln!(
            "{}番目のレストラン: p_{} = ({}, {})",
            i, nearest_restaurant, restaurant_pos.x, restaurant_pos.y
        );
    }

    // 【ヒント】ここまで穴埋めできたら、正しく動くか一度実行してみましょう！

    // 3.受けた注文を捌ききるまで、今いる場所から一番近い配達先に移動することを繰り返す

    // 行かなければいけない配達先のリスト
    // ordersは最終的に出力しなければならないので、ここでコピーを作成しておく
    // 配達先を訪問したらこのリストから1つずつ削除していく
    let mut destinations = orders.clone();

    // pickup_count(=50)回ループ
    for i in 0..input.pickup_count {
        // 配達先を全探索して、最も近い配達先を探す
        let mut nearest_index = 0; // 配達先リストのインデックス
        let mut nearest_destination = destinations[nearest_index]; // 配達先の番号
        let mut min_dist = i32::MAX; // 最も近い配達先の距離

        // 0～999まで全探索するのではなく、50個のレストランに対応した配達先を全探索することに注意
        for j in 0..destinations.len() {
            // 【穴埋め】最短距離が更新されたら記録
            // 【ヒント】nearest_index, nearest_destination, min_distの3つを更新する
            /* put your code here */
        }

        // 最も近い配達先(nearest_destination)に移動する
        // 【穴埋め】現在位置を最も近い配達先の位置に更新
        /* put your code here */

        // 【穴埋め】配達ルートに現在の位置を追加
        /* put your code here */

        // 【穴埋め】配達先のリストから削除
        /* put your code here */

        // 総移動距離の更新
        total_dist += min_dist;

        // デバッグしやすいよう、標準エラー出力に配達先を出力
        let destination_pos = input.destinations[nearest_destination as usize];
        eprintln!(
            "{}番目の配達先: q_{} = ({}, {})",
            i, nearest_destination, destination_pos.x, destination_pos.y
        );
    }

    // 4. オフィスに戻る
    route.push(input.office);
    total_dist += current_position.dist(input.office);

    // 合計距離を標準エラー出力に出力
    eprintln!("total distance: {}", total_dist);

    Output::new(orders, route)
}

fn main() {
    // 入力データを読み込む
    let input = Input::read();

    // 問題を解く
    let output = solve(&input);

    // 出力する
    output.print();
}
