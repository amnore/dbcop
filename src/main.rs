mod clients;
mod db;

use clap::{App, AppSettings, Arg, SubCommand};
use clients::{DynCluster, DynNode, MemgraphCluster, PostgresCluster};
use db::cluster::{Cluster, ClusterNode};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use rand::distributions::{Bernoulli, Distribution, Uniform};

use std::path::Path;

use std::fs;

use db::distribution::{MyDistribution, MyDistributionTrait};
use db::history::generate_mult_histories;
use db::history::History;

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
                        .short('d')
                        .takes_value(true)
                        .required(true)
                        .help("Directory to generate histories"),
                )
                .arg(
                    Arg::with_name("n_history")
                        .long("nhist")
                        .required(true)
                        .takes_value(true)
                        .help("Number of histories to generate"),
                )
                .arg(
                    Arg::with_name("n_node")
                        .long("nnode")
                        .short('n')
                        .required(true)
                        .takes_value(true)
                        .help("Number of nodes per history"),
                )
                .arg(
                    Arg::with_name("n_variable")
                        .long("nvar")
                        .short('v')
                        .required(true)
                        .takes_value(true)
                        .help("Number of variables per history"),
                )
                .arg(
                    Arg::with_name("n_transaction")
                        .long("ntxn")
                        .short('t')
                        .required(true)
                        .takes_value(true)
                        .help("Number of transactions per history"),
                )
                .arg(
                    Arg::with_name("n_event")
                        .long("nevt")
                        .short('e')
                        .required(true)
                        .takes_value(true)
                        .help("Number of events per transactions"),
                )
                .arg(
                    Arg::with_name("read_probability")
                        .long("readp")
                        .required(true)
                        .takes_value(true)
                        .help("Probability for an event to be a read"),
                )
                .arg(
                    Arg::with_name("key_distribution")
                        .long("key_distrib")
                        .required(true)
                        .takes_value(true)
                        .possible_values(["uniform", "zipf", "hotspot"])
                        .help("Key access distribution"),
                )
                .about("Generate histories"),
            SubCommand::with_name("print").arg(
                Arg::with_name("directory")
                    .short('d')
                    .takes_value(true)
                    .help("Directory containing executed history"),
            ),
            SubCommand::with_name("run")
                .about("Execute operations on db")
                .arg(
                    Arg::with_name("hist_dir")
                        .long("dir")
                        .short('d')
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("hist_out")
                        .long("out")
                        .short('o')
                        .takes_value(true)
                        .required(true),
                )
                .arg(Arg::with_name("ip:port").help("DB addr").required(true))
                .arg(
                    Arg::with_name("database")
                        .long("db")
                        .takes_value(true)
                        .possible_values(["memgraph", "postgres"])
                        .required(true),
                ),
        ])
        .setting(AppSettings::SubcommandRequired);

    let app_matches = app.get_matches();

    match app_matches.subcommand() {
        Some(("print", m)) => {
            let v_path = Path::new(m.value_of("directory").unwrap()).join("history.bincode");
            let file = File::open(v_path).unwrap();
            let buf_reader = BufReader::new(file);
            let hist: History = bincode::deserialize_from(buf_reader).unwrap();

            println!("{:?}", hist);
        }
        Some(("generate", matches)) => {
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
        Some(("run", matches)) => {
            let hist_dir = Path::new(matches.value_of("hist_dir").unwrap());
            let hist_out = Path::new(matches.value_of("hist_out").unwrap());

            fs::create_dir_all(hist_out).expect("couldn't create directory");

            let ips: Vec<_> = matches.values_of("ip:port").unwrap().collect();

            let mut cluster: Box<dyn Cluster<DynNode>> = match matches.value_of("database") {
                Some("memgraph") => Box::new(DynCluster::new(MemgraphCluster::new(&ips))),
                Some("postgres") => Box::new(DynCluster::new(PostgresCluster::new(&ips))),
                _ => unreachable!(),
            };

            cluster.execute_all(hist_dir, hist_out, 100);
        }
        _ => unreachable!(),
    }
}
