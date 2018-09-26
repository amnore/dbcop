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
    println!("{:?}", hist);
    let mut id_vec = Vec::new();
    id_vec.push((0, 0, 0));
    for (i_node, session) in hist.iter().enumerate() {
        for (i_transaction, transaction) in session.iter().enumerate() {
            for (i_event, event) in transaction.events.iter().enumerate() {
                assert_eq!(event.id, id_vec.len());
                id_vec.push((i_node + 1, i_transaction, i_event));
            }
        }
    }

    transactional_history_verify(&hist, &id_vec);
}
