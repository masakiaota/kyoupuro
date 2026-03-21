//! 00_template.rs
//! AtCoder Heuristic First-step Vol.1 のRust向けテンプレートファイル
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
    // サンプル解法
    // 以下を順に実行するプログラム
    // 1.高橋君は最初オフィスから出発する
    // 2.0番目のレストラン, 0番目の配達先, ..., 49番目のレストラン, 49番目の配達先の順に移動する
    // 3.オフィスに帰る
    let mut orders = vec![]; // 注文の集合
    let mut route = vec![]; // 配達ルート

    // 1. オフィスからスタート
    route.push(input.office);
    let mut current_position = input.office; // 現在地
    let mut total_dist = 0; // 総移動距離

    // 2.レストランと配達先を50箇所（pickup_count）ずつ巡る
    for i in 0..input.pickup_count {
        // 次のレストランに移動する
        // 注文の集合にi番目のレストランを追加
        orders.push(i);

        // 配達ルートにi番目のレストランの位置を追加
        route.push(input.restaurants[i]);

        // 総移動距離の更新
        total_dist += current_position.dist(input.restaurants[i]);

        // 現在位置をi番目のレストランの位置に更新
        current_position = input.restaurants[i];

        // 次の配達先に移動する
        // 配達ルートにi番目の配達先の位置を追加
        route.push(input.destinations[i]);

        // 総移動距離の更新
        total_dist += current_position.dist(input.destinations[i]);

        // 現在位置を最も近い配達先の位置に更新
        current_position = input.destinations[i];
    }

    // 3. オフィスに戻る
    route.push(input.office);
    total_dist += current_position.dist(input.office);

    // 合計距離を標準エラー出力に出力
    // 標準エラー出力はデバッグに有効なので、AHCでは積極的に活用していきましょう
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
