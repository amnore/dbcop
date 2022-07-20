extern crate clap;
extern crate dbcop;

// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use clap::{App, AppSettings, Arg, SubCommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use std::path::Path;

use std::fs;

use dbcop::db::history::generate_mult_histories;
use dbcop::db::history::History;

fn main() {
    let app = App::new("dbcop")
        .version("1.0")
        .author("Ranadeep")
        .about("Generates histories or verifies executed histories")
        .subcommands(vec![
            SubCommand::with_name("generate")
                .arg(
                    Arg::with_name("g_directory")
                        .long("gen_dir")
                        .short("d")
                        .takes_value(true)
                        .required(true)
                        .help("Directory to generate histories"),
                )
                .arg(
                    Arg::with_name("n_history")
                        .long("nhist")
                        .short("h")
                        .default_value("10")
                        .help("Number of histories to generate"),
                )
                .arg(
                    Arg::with_name("n_node")
                        .long("nnode")
                        .short("n")
                        .default_value("3")
                        .help("Number of nodes per history"),
                )
                .arg(
                    Arg::with_name("n_variable")
                        .long("nvar")
                        .short("v")
                        .default_value("5")
                        .help("Number of variables per history"),
                )
                .arg(
                    Arg::with_name("n_transaction")
                        .long("ntxn")
                        .short("t")
                        .default_value("5")
                        .help("Number of transactions per history"),
                )
                .arg(
                    Arg::with_name("n_event")
                        .long("nevt")
                        .short("e")
                        .default_value("2")
                        .help("Number of events per transactions"),
                )
                .about("Generate histories"),
            SubCommand::with_name("print")
                .arg(
                    Arg::with_name("directory")
                        .short("d")
                        .takes_value(true)
                        .help("Directory containing executed history")
                )
        ])
        .setting(AppSettings::SubcommandRequired);

    let app_matches = app.get_matches();

    match app_matches.subcommand() {
        ("print", Some(m)) => {
            let v_path =
                Path::new(m.value_of("directory").unwrap()).join("history.bincode");
            let file = File::open(v_path).unwrap();
            let buf_reader = BufReader::new(file);
            let hist: History = bincode::deserialize_from(buf_reader).unwrap();

            println!("{:?}", hist);
        }
        ("generate", Some(matches)) => {
            let dir = Path::new(matches.value_of("g_directory").unwrap());

            if !dir.is_dir() {
                fs::create_dir_all(dir).expect("failed to create directory");
            }

            let mut histories = generate_mult_histories(
                matches.value_of("n_history").unwrap().parse().unwrap(),
                matches.value_of("n_node").unwrap().parse().unwrap(),
                matches.value_of("n_variable").unwrap().parse().unwrap(),
                matches.value_of("n_transaction").unwrap().parse().unwrap(),
                matches.value_of("n_event").unwrap().parse().unwrap(),
            );

            for hist in histories.drain(..) {
                let file = File::create(dir.join(format!("hist-{:05}.bincode", hist.get_id())))
                    .expect("couldn't create bincode file");
                let buf_writer = BufWriter::new(file);
                bincode::serialize_into(buf_writer, &hist)
                    .expect("dumping history to bincode file went wrong");
            }
        }
        _ => unreachable!(),
    }
}
