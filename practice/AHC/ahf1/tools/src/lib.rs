#![allow(non_snake_case, unused_imports, unused_macros)]

use rand::prelude::*;
use std::{collections::{BTreeMap, BTreeSet}, io::prelude::*};
use proconio::{input, marker::*, source::Source};
use svg::node::element::{Circle, Path, Rectangle, path::Data};

pub trait SetMinMax {
	fn setmin(&mut self, v: Self) -> bool;
	fn setmax(&mut self, v: Self) -> bool;
}
impl<T> SetMinMax for T where T: PartialOrd {
	fn setmin(&mut self, v: T) -> bool {
		*self > v && { *self = v; true }
	}
	fn setmax(&mut self, v: T) -> bool {
		*self < v && { *self = v; true }
	}
}

#[macro_export]
macro_rules! mat {
	($($e:expr),*) => { Vec::from(vec![$($e),*]) };
	($($e:expr,)*) => { Vec::from(vec![$($e),*]) };
	($e:expr; $d:expr) => { Vec::from(vec![$e; $d]) };
	($e:expr; $d:expr $(; $ds:expr)+) => { Vec::from(vec![mat![$e $(; $ds)*]; $d]) };
}

#[derive(Clone)]
pub struct Output {
	r: Vec<usize>,
	path: Vec<(i32, i32)>,
}

const N: usize = 1000;
const M: usize = 50;

pub struct Input {
	from: Vec<(i32, i32)>,
	to: Vec<(i32, i32)>,
}

impl std::fmt::Display for Input {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for i in 0..N {
			writeln!(f, "{} {} {} {}", self.from[i].0, self.from[i].1, self.to[i].0, self.to[i].1)?;
		}
		Ok(())
	}
}

pub fn parse_input(f: &str) -> Input {
	let f = proconio::source::once::OnceSource::from(f);
	input! {
		from f,
		a: [(i32, i32, i32, i32); N]
	}
	let from = a.iter().map(|&(x, y, _, _)| (x, y)).collect();
	let to = a.iter().map(|&(_, _, x, y)| (x, y)).collect();
	Input { from, to }
}

pub fn parse_output(_input: &Input, f: &str) -> Vec<Output> {
	let mut out = vec![];
	let mut f = proconio::source::once::OnceSource::from(f);
	while !f.is_empty() {
		input! {
			from &mut f,
			r: [usize],
			path: [(i32, i32)],
		}
		out.push(Output { r: r.into_iter().map(|i| i - 1).collect(), path });
	}
	out
}

fn dist((x1, y1): (i32, i32), (x2, y2): (i32, i32)) -> i32 {
	(x1 - x2).abs() + (y1 - y2).abs()
}

pub fn compute_score(input: &Input, out: &Output) -> (i64, String, i64) {
	let mut time = 0;
	for i in 1..out.path.len() {
		time += dist(out.path[i - 1], out.path[i]) as i64;
	}
	for i in 0..out.r.len() {
		if out.r[i] >= N {
			return (0, format!("Illegal output (r[{}] = {})", i + 1, out.r[i] + 1), time);
		}
		for j in 0..i {
			if out.r[i] == out.r[j] {
				return (0, format!("Illegal output (r[{}] = r[{}])", i + 1, j + 1), time);
			}
		}
	}
	for i in 0..out.path.len() {
		if out.path[i].0 < 0 || out.path[i].0 > 800 || out.path[i].1 < 0 || out.path[i].1 > 800	{
			return (0, "Illegal output".to_owned(), time);
		}
	}
	if out.path.len() == 0 || out.path[0] != (400, 400) {
		return (0, "Illegal output (x[1],y[1]) != (400, 400)".to_owned(), time);
	} else if out.path[out.path.len() - 1] != (400, 400) {
		return (0, "Illegal output (x[n],y[n]) != (400, 400)".to_owned(), time);
	}
	let mut first_visit = BTreeMap::new();
	let mut last_visit = BTreeMap::new();
	for i in 0..out.path.len() {
		first_visit.entry(out.path[i]).or_insert(i);
		last_visit.insert(out.path[i], i);
	}
	for &i in &out.r {
		if let (Some(first), Some(last)) = (first_visit.get(&input.from[i]), last_visit.get(&input.to[i])) {
			if first >= last {
				return (0, format!("{}-th delivery has not been completed", i + 1), time);
			}
		} else {
			return (0, format!("{}-th delivery has not been completed", i + 1), time);
		}
	}
	if out.r.len() != M {
		return (0, "Illegal output (m != 50)".to_owned(), time);
	}
	let score = (1e8 / (1000 + time) as f64).round() as i64;
	(score, String::new(), time)
}

pub fn gen(seed: u64) -> Input {
	let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
	let mut from = vec![];
	let mut to = vec![];
	while from.len() < N {
		let x1 = rng.gen_range(0, 801);
		let y1 = rng.gen_range(0, 801);
		let x2 = rng.gen_range(0, 801);
		let y2 = rng.gen_range(0, 801);
		if dist((x1, y1), (x2, y2)) < 100 {
			continue;
		}
		from.push((x1, y1));
		to.push((x2, y2));
	}
	Input {
		from,
		to,
	}
}

fn rect(x: i32, y: i32, w: i32, h: i32, fill: &str) -> Rectangle {
	Rectangle::new().set("x", x).set("y", y).set("width", w).set("height", h).set("fill", fill)
}

pub fn get_max_t(out: &Output) -> i32 {
	let mut time = 0;
	for i in 1..out.path.len() {
		time += dist(out.path[i - 1], out.path[i]);
	}
	time
}

pub fn vis_default(input: &Input, out: &Output) -> String {
	vis(input, out, get_max_t(out), true, false)
}

pub fn vis(input: &Input, out: &Output, t: i32, show_pair: bool, show_all: bool) -> String {
	let mut doc = svg::Document::new().set("id", "vis").set("viewBox", (-5, -5, 810, 810)).set("width", 810).set("height", 810);
	doc = doc.add(rect(-5, -5, 810, 810, "white"));
	let mut status = vec![!0; N];
	let mut ps = BTreeMap::<(i32, i32), Vec<(usize, usize)>>::new();
	for &i in &out.r {
		status[i] = 0;
		ps.entry(input.from[i]).or_default().push((i, 0));
		ps.entry(input.to[i]).or_default().push((i, 1));
	}
	if show_pair {
		for i in 0..N {
			if status[i] != !0 || show_all {
				let data = Data::new().move_to(input.from[i]).line_to(input.to[i]);
				let path = Path::new().set("stroke", "lightgray").set("stroke-width", 1).set("d", data);
				doc = doc.add(path);
			}
		}
	}
	let mut time = 0;
	for i in 0..out.path.len() {
		if i > 0 {
			let d = dist(out.path[i - 1], out.path[i]);
			let data = if time + d > t {
				let len = d.min(t - time);
				let mul = len as f64 / d as f64;
				Data::new().move_to(out.path[i - 1]).line_by(((out.path[i].0 - out.path[i - 1].0) as f64 * mul, (out.path[i].1 - out.path[i - 1].1) as f64 * mul))
			} else {
				Data::new().move_to(out.path[i - 1]).line_to(out.path[i])
			};
			let path = Path::new().set("stroke", "brown").set("stroke-width", 3).set("d", data);
			doc = doc.add(path);
			time += d;
		}
		if time > t {
			break;
		}
		for &(j, k) in ps.get(&out.path[i]).unwrap_or(&vec![]) {
			if status[j] == k {
				status[j] = k + 1;
			}
		}
	}
	for i in 0..N {
		if status[i] != !0 {
			let c = if status[i] >= 1 {
				"red"
			} else {
				"red"
			};
			let circle = Circle::new().set("cx", input.from[i].0).set("cy", input.from[i].1).set("r", 6).set("fill", c);
			doc = doc.add(circle);
			let c = if status[i] == 2 {
				"green"
			} else if status[i] == 1 {
				"green"
			} else {
				"blue"
			};
			let circle = Circle::new().set("cx", input.to[i].0).set("cy", input.to[i].1).set("r", 6).set("fill", c);
			doc = doc.add(circle);
		} else if show_all {
			let circle = Circle::new().set("cx", input.from[i].0).set("cy", input.from[i].1).set("r", 3).set("fill", "lightpink");
			doc = doc.add(circle);
			let circle = Circle::new().set("cx", input.to[i].0).set("cy", input.to[i].1).set("r", 3).set("fill", "lightblue");
			doc = doc.add(circle);
		}
	}
	let circle = Circle::new().set("cx", 400).set("cy", 400).set("r", 6).set("fill", "gray");
	doc = doc.add(circle);
	doc.to_string()
}
