//! 02_hill_climbing.rs
//! 貪欲法で初期解を求めた後、配達先の訪問順序を山登り法で改善する解法プログラム
//! 必要なクレートの導入には cargo add proconio@=0.4.5 rand@=0.8.5 rand_pcg@=0.3.1 を実行してください
use proconio::input;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::{
    fmt::Display,
    time::{Duration, Instant},
};

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

/// route の総移動距離を計算
fn get_distance(route: &Vec<Point>) -> i32 {
    let mut dist = 0;

    for i in 0..route.len() - 1 {
        dist += route[i].dist(route[i + 1]);
    }

    dist
}

/// 問題を解く関数（この関数を実装していきます）
fn solve_greedy(input: &Input) -> Output {
    // 貪欲その2
    // 以下を順に実行するプログラム
    // 1.オフィスから距離400以下の注文だけを候補にする
    // 2.高橋君は最初オフィスから出発する
    // 3.訪問したレストランが50軒に達するまで、今いる場所から一番近いレストランに移動することを繰り返す
    // 4.受けた注文を捌ききるまで、今いる場所から一番近い配達先に移動することを繰り返す
    // 5.オフィスに帰る

    let mut candidates = vec![]; // 注文の候補
    let mut orders = vec![]; // 注文の集合
    let mut route = vec![]; // 配達ルート

    // 1. オフィスから距離400以下の注文だけを候補にする
    for i in 0..input.order_count {
        if input.office.dist(input.restaurants[i]) <= 400
            && input.office.dist(input.destinations[i]) <= 400
        {
            candidates.push(i);
        }
    }

    // 2.オフィスからスタート
    route.push(input.office);
    let mut current_position = input.office; // 現在地
    let mut total_dist = 0; // 総移動距離

    // 3.訪問したレストランが50軒に達するまで、今いる場所から一番近いレストランに移動することを繰り返す

    // 同じレストランを2回訪れてはいけないので、訪問済みのレストランを記録する
    let mut visited_restaurant = vec![false; input.order_count];

    // pickup_count(=50)回ループ
    for i in 0..input.pickup_count {
        // レストランを全探索して、最も近いレストランを探す
        let mut nearest_restaurant = 0; // レストランの番号
        let mut min_dist = 1000000; // 最も近いレストランの距離

        // 候補にした注文だけを調べる
        for &j in &candidates {
            // 既に訪れていたらスキップ
            if visited_restaurant[j] {
                continue;
            }

            // 最短距離が更新されたら記録
            let distance = current_position.dist(input.restaurants[j]);

            if distance < min_dist {
                min_dist = distance;
                nearest_restaurant = j;
            }
        }

        // 最も近いレストラン(nearest_restaurant)に移動する
        // 現在位置を最も近いレストランの位置に更新
        current_position = input.restaurants[nearest_restaurant];

        // 注文の集合に選んだレストランを追加
        orders.push(nearest_restaurant);

        // 配達ルートに現在の位置を追加
        route.push(current_position);

        // 訪問済みレストランの配列にtrueをセット
        visited_restaurant[nearest_restaurant] = true;

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

    // 4.受けた注文を捌ききるまで、今いる場所から一番近い配達先に移動することを繰り返す

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
            // 最短距離が更新されたら記録
            let distance = current_position.dist(input.destinations[destinations[j]]);

            if distance < min_dist {
                min_dist = distance;
                nearest_index = j;
                nearest_destination = destinations[j];
            }
        }

        // 最も近い配達先(nearest_destination)に移動する
        // 現在位置を最も近い配達先の位置に更新
        current_position = input.destinations[nearest_destination];

        // 配達ルートに現在の位置を追加
        route.push(current_position);

        // 配達先のリストから削除
        destinations.remove(nearest_index);

        // 総移動距離の更新
        total_dist += min_dist;

        // デバッグしやすいよう、標準エラー出力に配達先を出力
        let destination_pos = input.destinations[nearest_destination as usize];
        eprintln!(
            "{}番目の配達先: q_{} = ({}, {})",
            i, nearest_destination, destination_pos.x, destination_pos.y
        );
    }

    // 5.オフィスに戻る
    route.push(input.office);
    total_dist += current_position.dist(input.office);

    // 合計距離を標準エラー出力に出力
    eprintln!("total distance: {}", total_dist);

    Output::new(orders, route)
}

/// 配達先の訪問順序を山登り法で改善する関数（この関数を実装していきます）
fn solve_hill_climbing(input: &Input, output_greedy: &Output) -> Output {
    // 山登り法
    // 「ある1つの配達先を訪問する順序を、別の場所に入れ替える」操作を繰り返すことで、経路を改善する

    // 貪欲法で求めた解をコピー(これを初期解とする)
    let orders = output_greedy.orders.clone();
    let mut route = output_greedy.route.clone();

    // 現在の経路の距離を計算
    let mut current_dist = get_distance(&route);

    // 乱数生成器を用意
    // 乱数のシード値は固定のものにしておくと、デバッグがしやすくなります
    let mut rand = Pcg64Mcg::new(42);

    // 山登り法の開始時刻を取得
    let start_time = Instant::now();

    // 制限時間(1.9秒)
    // 2秒ちょうどまでやるとTLEになるので、1.9秒程度にしておくとよい
    let time_limit = Duration::from_millis(1900);

    // 試行回数
    let mut iteration = 0;

    // 山登り法の本体
    loop {
        // 現在時刻を取得
        let elapsed = start_time.elapsed();

        // 制限時間になったら終了
        if elapsed >= time_limit {
            break;
        }

        // 訪問先が配達先であるようなインデックスの中から、
        // 「i番目の訪問先をj番目に移動」する操作をランダムに選ぶことで、
        // ある配達先を訪れる順序を他の配達先の間に変える
        // 貪欲法で求めた解では、配達先の訪問順序は0-indexedで51番目～100番目であることに注意
        // (AtCoderオフィス、レストラン50軒、配達先50軒、AtCoderオフィスの順に並んでいる)

        // 【穴埋め】訪問先が配達先であるようなインデックスの中から i, j をランダムに選ぶ
        // 【ヒント】let i = rand.gen_range(a..b); と書くと、a以上b未満の乱数が得られる
        let i = rand.gen_range(0..input.pickup_count) + input.pickup_count + 1;
        let j = rand.gen_range(0..input.pickup_count) + input.pickup_count + 1;

        // 【穴埋め】i番目の訪問先をj番目に移動する操作を行う
        // 【ヒント】routeのi番目の要素を削除した後、削除した要素をj番目に挿入することで移動する操作になる
        let point_to_move = route.remove(i);
        route.insert(j, point_to_move);

        // 【穴埋め】操作後の経路の距離を計算
        // 【ヒント】get_distance(&r)を使うと、経路rの距離が計算できる
        let new_dist = get_distance(&route);

        // 【穴埋め】操作後の距離が現在(操作前)の距離以下なら採用
        // 【ヒント】現在の距離はcurrent_distに入っている
        if new_dist <= current_dist {
            // 進行状況を可視化するため、距離が真に小さくなったら、現在の試行回数と合計距離を標準エラー出力に出力
            if new_dist < current_dist {
                eprintln!("iteration: {}, total distance: {}", iteration, new_dist);
            }

            // 【穴埋め】現在の距離を操作後の距離で更新
            current_dist = new_dist;
        } else {
            // 【穴埋め】操作前より悪化していたら元に戻す
            // 【ヒント】「i番目の訪問先をj番目に移動する操作」を元に戻すには「j番目の訪問先をi番目に移動する操作」を行えばよい
            let point_to_move = route.remove(j);
            route.insert(i, point_to_move);
        }

        // 試行回数のカウントを増やす
        iteration += 1;
    }

    // 試行回数と合計距離を標準エラー出力に出力
    eprintln!("--- Result ---");
    eprintln!("iteration     : {}", iteration);
    eprintln!("total distance: {}", current_dist);

    Output::new(orders, route)
}

fn main() {
    // 入力データを読み込む
    let input = Input::read();

    // 問題を解く
    let output = solve_greedy(&input);
    let output = solve_hill_climbing(&input, &output);

    // 出力する
    output.print();
}
