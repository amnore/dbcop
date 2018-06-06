#![allow(dead_code)]

extern crate mysql;
extern crate rand;

mod algo;
mod consistency;
mod db;

fn main() {
    algo::bench::do_bench();
}
