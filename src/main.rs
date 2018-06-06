#![allow(dead_code)]

extern crate clap;
extern crate mysql;
extern crate rand;
extern crate time;

mod algo;
mod db;

use clap::{App, Arg};

fn main() {
    // let matches = App::new("DBcop")
    //     .arg(
    //         Arg::with_name("mysql_ip")
    //             .short("i")
    //             .long("ip")
    //             .help("MySQL ip")
    //             .takes_value(true),
    //     )
    //     .arg(
    //         Arg::with_name("mysql_port")
    //             .short("p")
    //             .long("port")
    //             .help("MySQL port")
    //             .takes_value(true),
    //     )
    //     .arg(
    //         Arg::with_name("mysql_username")
    //             .short("u")
    //             .long("username")
    //             .help("MySQL username")
    //             .required(true)
    //             .takes_value(true),
    //     )
    //     .arg(
    //         Arg::with_name("mysql_secret")
    //             .short("s")
    //             .long("secret")
    //             .help("MySQL secret")
    //             .required(true)
    //             .takes_value(true),
    //     )
    //     .get_matches();
    //
    // let ip = matches.value_of("mysql_ip").unwrap_or("localhost");
    // let port = matches.value_of("mysql_port").unwrap_or("3306");
    // let user = matches.value_of("mysql_username").unwrap();
    // let sec = matches.value_of("mysql_secret").unwrap();
    // println!("{} {} {:?} {:?}", ip, port, user, sec);

    // let conn_str = format!("mysql://{}:{}@{}:{}", user, sec, ip, port);

    // println!("{}", conn_str);

    algo::bench::do_bench();
}
