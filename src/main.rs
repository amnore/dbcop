mod clients;
mod db;
mod verifier;

use clap::{Parser, Subcommand, ValueEnum};
use clients::{DynCluster, DynNode, MemgraphCluster, PostgresCluster, PostgresSERCluster, DGraphCluster, GaleraCluster, MySQLCluster};
use db::cluster::Cluster;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use rand::distributions::{Bernoulli, Distribution, Uniform};

use std::path::PathBuf;

use std::fs;

use db::distribution::{MyDistribution, MyDistributionTrait};
use db::history::{generate_mult_histories, HistoryParams};
use db::history::History;

use verifier::Verifier;

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

#[derive(Parser)]
#[clap(name = "dbcop", author = "Ranadeep", about = "Generates histories or verifies executed histories")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Generate histories")]
    Generate {
        #[clap(short = 'd', long = "gen_dir", help = "Directory to generate histories")]
        g_directory: PathBuf,

        #[clap(long = "nhist", default_value_t = 1, help = "Number of histories to generate")]
        n_history: usize,

        #[clap(long = "nnode", short = 'n', help = "Number of nodes per history")]
        n_node: usize,

        #[clap(long = "nvar", short = 'v', help = "Number of variables per history")]
        n_variable: usize,

        #[clap(long = "ntxn", short = 't', help = "Number of transactions per history")]
        n_transaction: usize,

        #[clap(long = "nevt", short = 'e', help = "Number of events per transactions")]
        n_event: usize,

        #[clap(long = "readp", default_value_t = 0.5, help = "Probability for an event to be a read")]
        read_probability: f64,

        #[clap(value_enum, long = "key_distrib", default_value_t = KeyDistribution::Uniform, help = "Key access distribution")]
        key_distribution: KeyDistribution,

        #[clap(long, default_value_t = 0.0, help = "Proportion of long transactions")]
        longtxn_proportion: f64,

        #[clap(long, default_value_t = 10.0, help = "Times of size of long transactions compared to regular txns")]
        longtxn_size: f64,
    },
    Print {
        #[clap(short = 'd', help = "Directory containing executed history")]
        directory: PathBuf,
    },
    #[clap(about = "Execute operations on db")]
    Run {
        #[clap(long = "dir", short = 'd')]
        hist_dir: PathBuf,

        #[clap(long = "out", short = 'o')]
        hist_out: PathBuf,

        #[clap(value_name = "ip:port", help = "DB addr")]
        addrs: Vec<String>,

        #[clap(long = "db", value_enum)]
        database: Database,
    },
    #[clap(about = "Verifies histories")]
    Verify {
        #[clap(long = "ver_dir", short = 'd', help = "Directory containing executed histories")]
        v_directory: PathBuf,

        #[clap(long = "out_dir", short = 'o', help = "Directory to output the results")]
        o_directory: PathBuf,

        #[clap(long = "sat", default_value_t = false, help = "Use MiniSAT as backend")]
        sat: bool,

        #[clap(long = "bic", default_value_t = false, help = "Use BiComponent")]
        bicomponent: bool,

        #[clap(long = "cons", short = 'c', value_enum, help = "Check for mentioned consistency")]
        consistency: Option<Consistency>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum KeyDistribution {
    Uniform, Zipf, Hotspot
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Database {
    Memgraph, Postgres, PostgresSer, Dgraph, Galera, Mysql
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Consistency {
    Cc, Si, Ser
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Print { directory } => {
            let v_path = directory.join("history.bincode");
            let file = File::open(v_path).unwrap();
            let buf_reader = BufReader::new(file);
            let hist: History = bincode::deserialize_from(buf_reader).unwrap();

            println!("{:?}", hist);
        }
        Commands::Generate { g_directory, n_history, n_node, n_variable, n_transaction, n_event, read_probability, key_distribution, longtxn_proportion, longtxn_size } => {
            if !g_directory.is_dir() {
                fs::create_dir_all(&g_directory).expect("failed to create directory");
            }

            let distribution: Box<dyn MyDistributionTrait> =
                match key_distribution {
                    KeyDistribution::Uniform => Box::new(MyDistribution::new(Uniform::new(0, n_variable))),
                    KeyDistribution::Zipf => Box::new(MyDistribution::new(
                        ZipfDistribution::new(n_variable, 0.5)
                            .unwrap()
                            .map(|x| x - 1),
                    )),
                    KeyDistribution::Hotspot => {
                        Box::new(MyDistribution::new(HotspotDistribution::new(n_variable)))
                    }
                };

            let mut histories = generate_mult_histories(
                HistoryParams {
                    n_hist: n_history,
                    n_node,
                    n_variable,
                    n_transaction,
                    n_event,
                    read_probability,
                    key_distribution: distribution.as_ref(),
                    longtxn_proportion,
                    longtxn_size,
                }
            );

            for hist in histories.drain(..) {
                let file = File::create(g_directory.join(format!("hist-{:05}.bincode", hist.get_id())))
                    .expect("couldn't create bincode file");
                let buf_writer = BufWriter::new(file);
                bincode::serialize_into(buf_writer, &hist)
                    .expect("dumping history to bincode file went wrong");
            }
        }
        Commands::Run { hist_dir, hist_out, addrs, database } => {
            fs::create_dir_all(&hist_out).expect("couldn't create directory");
            let addrs_str = addrs.iter().map(|addr| addr.as_str()).collect();

            let mut cluster: Box<dyn Cluster<DynNode>> = match database {
                Database::Memgraph => Box::new(DynCluster::new(MemgraphCluster::new(&addrs_str))),
                Database::Postgres => Box::new(DynCluster::new(PostgresCluster::new(&addrs_str))),
                Database::PostgresSer => Box::new(DynCluster::new(PostgresSERCluster::new(&addrs_str))),
                Database::Dgraph => Box::new(DynCluster::new(DGraphCluster::new(&addrs_str))),
                Database::Galera => Box::new(DynCluster::new(GaleraCluster::new(&addrs_str))),
                Database::Mysql => Box::new(DynCluster::new(MySQLCluster::new(&addrs_str))),
            };

            cluster.execute_all(&hist_dir.as_path(), &hist_out.as_path(), 100);
        }
        Commands::Verify { v_directory, o_directory, sat, bicomponent, consistency } => {
            let v_path = v_directory.join("history.bincode");
            let file = File::open(v_path).unwrap();
            let buf_reader = BufReader::new(file);
            let hist: History = bincode::deserialize_from(buf_reader).unwrap();

            println!("{:?}", hist);

            if !o_directory.is_dir() {
                fs::create_dir_all(&o_directory).expect("failed to create directory");
            }

            // let curr_dir = o_dir.join(format!("hist-{:05}", hist.get_id()));

            let mut verifier = Verifier::new(&o_directory);

            match consistency {
                Some(Consistency::Cc) => verifier.model("cc"),
                Some(Consistency::Si) => verifier.model("si"),
                Some(Consistency::Ser) => verifier.model("ser"),
                None => verifier.model(""),
            };

            verifier.sat(sat);
            verifier.bicomponent(bicomponent);

            println!("no. of session {:?}", hist.get_data().len());
            println!("no. of transactions {:?}", hist.get_data()[0].len());

            match verifier.verify(hist.get_data()) {
                Some(level) => println!(
                    "hist-{:05} failed - minimum level failed {:?}",
                    hist.get_id(),
                    level
                ),
                None => println!("hist-{:05} done", hist.get_id()),
            }
        }
    }
}
