extern crate clap;
extern crate dbcop;
extern crate rand;
extern crate zipf;

use clap::{App, AppSettings, Arg, SubCommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use rand::distributions::{Bernoulli, Distribution, Uniform};

use std::path::Path;

use std::fs;

use dbcop::db::distribution::{MyDistribution, MyDistributionTrait};
use dbcop::db::history::generate_mult_histories;
use dbcop::db::history::History;

use zipf::ZipfDistribution;

struct HotspotDistribution {
    hot_probability: Bernoulli,
    hot_key: Uniform<usize>,
    non_hot_key: Uniform<usize>,
}

impl Distribution<usize> for HotspotDistribution {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> usize {
        if self.hot_probability.sample(rng) {
            self.hot_key.sample(rng)
        } else {
            self.non_hot_key.sample(rng)
        }
    }
}

impl HotspotDistribution {
    fn new(n_variables: usize) -> HotspotDistribution {
        let hot_key_max = n_variables / 5;
        HotspotDistribution {
            hot_probability: Bernoulli::new(0.8).unwrap(),
            hot_key: Uniform::new(0, hot_key_max),
            non_hot_key: Uniform::new(hot_key_max, n_variables),
        }
    }
}

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
                .arg(
                    Arg::with_name("read_probability")
                        .long("readp")
                        .default_value("0.5")
                        .help("Probability for an event to be a read"),
                )
                .arg(
                    Arg::with_name("key_distribution")
                        .long("key_distrib")
                        .possible_values(&["uniform", "zipf", "hotspot"])
                        .default_value("uniform")
                        .help("Key access distribution"),
                )
                .about("Generate histories"),
            SubCommand::with_name("print").arg(
                Arg::with_name("directory")
                    .short("d")
                    .takes_value(true)
                    .help("Directory containing executed history"),
            ),
        ])
        .setting(AppSettings::SubcommandRequired);

    let app_matches = app.get_matches();

    match app_matches.subcommand() {
        ("print", Some(m)) => {
            let v_path = Path::new(m.value_of("directory").unwrap()).join("history.bincode");
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

            let n_variable = matches.value_of("n_variable").unwrap().parse().unwrap();
            let distribution: Box<dyn MyDistributionTrait> =
                match matches.value_of("key_distribution") {
                    Some("uniform") => Box::new(MyDistribution::new(Uniform::new(0, n_variable))),
                    Some("zipf") => Box::new(MyDistribution::new(
                        ZipfDistribution::new(n_variable, 0.5)
                            .unwrap()
                            .map(|x| x - 1),
                    )),
                    Some("hotspot") => {
                        Box::new(MyDistribution::new(HotspotDistribution::new(n_variable)))
                    }
                    _ => panic!(""),
                };

            let mut histories = generate_mult_histories(
                matches.value_of("n_history").unwrap().parse().unwrap(),
                matches.value_of("n_node").unwrap().parse().unwrap(),
                n_variable,
                matches.value_of("n_transaction").unwrap().parse().unwrap(),
                matches.value_of("n_event").unwrap().parse().unwrap(),
                matches
                    .value_of("read_probability")
                    .unwrap()
                    .parse()
                    .unwrap(),
                distribution.as_ref(),
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
