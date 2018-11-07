extern crate clap;
extern crate dbcop;
extern crate serde_yaml;

use clap::{App, Arg};
use std::fs::File;
use std::io::Read;

use dbcop::db::history::Transaction;
use dbcop::verifier::transactional_history_verify;

fn main() {
    let matches = App::new("dbcop")
        .version("1.0")
        .author("Ranadeep")
        .about("verifies a history")
        .arg(
            Arg::with_name("yaml_file")
                .long("yaml")
                .short("y")
                .value_name("YAML_FILE")
                .takes_value(true)
                .required(true)
                .help("yaml file containing history"),
        ).get_matches();

    let yaml_path = matches.value_of("yaml_file").unwrap();

    let mut bytes = Vec::new();
    let mut file = File::open(yaml_path).unwrap();
    file.read_to_end(&mut bytes).unwrap();
    let hist: Vec<Vec<Transaction>> = serde_yaml::from_slice(&bytes).unwrap();
    println!("{:#?}", hist);
    
    println!("{:?}", hist[0][1]);
    println!("{:?}", hist[0][2]);
    println!("{:?}", hist[2][13]);

    // transactional_history_verify(&hist);
}
