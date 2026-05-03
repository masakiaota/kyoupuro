// crate_check.rs
use ac_library as _;
use alga as _;
use amplify as _;
use amplify_derive as _;
use amplify_num as _;
use argio as _;
use ascii as _;
use az as _;
use bitset_fixed as _;
use bitvec as _;
use bstr as _;
use btreemultimap as _;
use counter as _;
use easy_ext as _;
use either as _;
use fixedbitset as _;
use getrandom as _;
use glidesort as _;
use hashbag as _;
use im_rc as _;
use indexing as _;
use indexmap as _;
use itertools as _;
use itertools_num as _;
use lazy_static as _;
use libm as _;
use maplit as _;
use memoise as _;
use multimap as _;
use multiversion as _;
use nalgebra as _;
use ndarray as _;
use num as _;
use num_bigint as _;
use num_complex as _;
use num_derive as _;
use num_integer as _;
use num_iter as _;
use num_rational as _;
use num_traits as _;
use omniswap as _;
use once_cell as _;
use ordered_float as _;
use pathfinding as _;
use permutohedron as _;
use petgraph as _;
use primal as _;
use proconio as _;
use rand as _;
use rand_chacha as _;
use rand_core as _;
use rand_distr as _;
use rand_hc as _;
use rand_pcg as _;
use rand_xorshift as _;
use rand_xoshiro as _;
use recur_fn as _;
use regex as _;
use rpds as _;
use rustc_hash as _;
use smallvec as _;
use static_assertions as _;
use statrs as _;
use superslice as _;
use tap as _;
use text_io as _;
use thiserror as _;
use varisat as _;

const CRATE_NAMES: &[&str] = &[
    "ac_library",
    "alga",
    "amplify",
    "amplify_derive",
    "amplify_num",
    "argio",
    "ascii",
    "az",
    "bitset_fixed",
    "bitvec",
    "bstr",
    "btreemultimap",
    "counter",
    "easy_ext",
    "either",
    "fixedbitset",
    "getrandom",
    "glidesort",
    "hashbag",
    "im_rc",
    "indexing",
    "indexmap",
    "itertools",
    "itertools_num",
    "lazy_static",
    "libm",
    "maplit",
    "memoise",
    "multimap",
    "multiversion",
    "nalgebra",
    "ndarray",
    "num",
    "num_bigint",
    "num_complex",
    "num_derive",
    "num_integer",
    "num_iter",
    "num_rational",
    "num_traits",
    "omniswap",
    "once_cell",
    "ordered_float",
    "pathfinding",
    "permutohedron",
    "petgraph",
    "primal",
    "proconio",
    "rand",
    "rand_chacha",
    "rand_core",
    "rand_distr",
    "rand_hc",
    "rand_pcg",
    "rand_xorshift",
    "rand_xoshiro",
    "recur_fn",
    "regex",
    "rpds",
    "rustc_hash",
    "smallvec",
    "static_assertions",
    "statrs",
    "superslice",
    "tap",
    "text_io",
    "thiserror",
    "varisat",
];

fn main() {
    println!("AtCoder Rust crate check");
    println!("If this program compiles, all listed crates are resolvable.");
    for crate_name in CRATE_NAMES {
        println!("{crate_name}: OK");
    }
    println!("total: {}", CRATE_NAMES.len());
}
