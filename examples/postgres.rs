extern crate clap;
extern crate dbcop;
extern crate postgres;
extern crate indicatif;

extern crate rand;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread::spawn;
use std::io::Write;

use dbcop::db::cluster::{Cluster, ClusterNode, Node};
use dbcop::db::history::{HistParams, Transaction};

use clap::{App, Arg};

use postgres::{Client, NoTls};

use indicatif::{MultiProgress, ProgressBar};

#[derive(Debug)]
pub struct PostgresNode {
    addr: String,
    id: usize,
    progress: Arc<MultiProgress>,
}

impl PostgresNode {
    fn new(node: Node, cluster: &PostgresCluster) -> Self {
        PostgresNode {
            addr: format!("postgresql://{}:{}@{}", "postgres", "postgres", node.addr),
            id: node.id,
            progress: cluster.1.clone(),
        }
    }
}

impl ClusterNode for PostgresNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let progress = self.progress.add(ProgressBar::new(hist.len() as u64));
        let mut conn = match Client::connect(self.addr.as_str(), NoTls) {
            Ok(conn) => conn,
            Err(e) => {
                assert_eq!(false, hist.iter().fold(false, |b, t| b | t.success));
                println!("CONNECTION ERROR {}", e);
                return;
            }
        };

        for transaction in progress.wrap_iter(hist.iter_mut()) {
            while !transaction.success {
                transaction.success = true;
                let mut sqltxn = match conn
                    .build_transaction()
                    .isolation_level(postgres::IsolationLevel::RepeatableRead)
                    .start()
                    {
                        Ok(txn) => txn,
                        Err(e) => {
                            println!("{:?} - TRANSACTION ERROR", e);
                            transaction.success = false;
                            continue;
                        }
                    };

                for event in transaction.events.iter_mut() {
                    if event.write {
                        match sqltxn.execute(
                            "UPDATE dbcop.variables SET val=$1 WHERE var=$2",
                            &[&(event.value as i64), &(event.variable as i64)],
                        ) {
                            Ok(_) => event.success = true,
                            Err(e) => {
                                // If an operation fails, then the whole transaction fails
                                transaction.success = false;
                                // eprintln!("WRITE ERR -- {:?}", e);
                                break;
                            }
                        }
                    } else {
                        match sqltxn.query(
                            "SELECT * FROM dbcop.variables WHERE var=$1",
                            &[&(event.variable as i64)],
                        ) {
                            Ok(result) => {
                                let row = result.get(0);
                                let value: i64 = row.unwrap().get("val");
                                event.value = value as usize;
                                event.success = true;
                            }
                            Err(e) => {
                                transaction.success = false;
                                // eprintln!("READ ERR -- {:?}", e);
                                break;
                            }
                        }
                    }
                }

                transaction.success &= match sqltxn.commit() {
                    Ok(_) => true,
                    Err(e) => {
                        // eprintln!("COMMIT ERR -- {:?}", e);
                        false
                    }
                };
            }
        }
    }
}

#[derive(Debug)]
pub struct PostgresCluster(Vec<Node>, Arc<MultiProgress>);

impl PostgresCluster {
    fn new(ips: &Vec<&str>) -> Self {
        PostgresCluster(PostgresCluster::node_vec(ips), Arc::new(MultiProgress::new()))
    }

    fn create_table(&self) -> bool {
        match self.get_postgresql_addr(0) {
            Some(ip) => Client::connect(ip.as_str(), NoTls)
                .and_then(|mut pool| {
                    pool.execute("CREATE SCHEMA IF NOT EXISTS dbcop",  &[]).unwrap();
                    pool.execute("DROP TABLE IF EXISTS dbcop.variables",  &[]).unwrap();
                    pool.batch_execute(
                        "CREATE TABLE IF NOT EXISTS dbcop.variables (var INT8 NOT NULL PRIMARY KEY, val INT8 NOT NULL) PARTITION BY HASH (var);
                         CREATE TABLE IF NOT EXISTS dbcop.variables_0 PARTITION OF dbcop.variables FOR VALUES WITH (modulus 3, remainder 0);
                         CREATE TABLE IF NOT EXISTS dbcop.variables_1 PARTITION OF dbcop.variables FOR VALUES WITH (modulus 3, remainder 1);
                         CREATE TABLE IF NOT EXISTS dbcop.variables_2 PARTITION OF dbcop.variables FOR VALUES WITH (modulus 3, remainder 2);"
                    ).unwrap();
                    // conn.query("USE dbcop").unwrap();
                    Ok(true)
                }).is_ok(),
            _ => false,
        }
    }

    fn create_variables(&self, n_variable: usize) {
        if let Some(ip) = self.get_postgresql_addr(0) {
            if let Ok(mut conn) = Client::connect(ip.as_str(), NoTls) {
                let mut writer = conn.copy_in("COPY dbcop.variables FROM STDIN").unwrap();
                (0..n_variable).for_each(|var| writer.write_all(format!("{}\t{}\n", var, 0).as_bytes()).unwrap());
                writer.finish().unwrap();
                // for stmt in conn
                    // .prepare("INSERT INTO dbcop.variables (var, val) values ($1, 0)")
                    // .into_iter()
                // {
                    // (0..n_variable).for_each(|variable| {
                        // conn.execute(&stmt, &[&(variable as i64)])
                            // .expect("Cannot create variable");
                    // });
                // }
            }
        }
    }

    fn drop_database(&self) {
        if let Some(ip) = self.get_postgresql_addr(0) {
            if let Ok(mut conn) = Client::connect(ip.as_str(), NoTls) {
                conn.execute("DROP SCHEMA dbcop CASCADE", &[]).unwrap();
            }
        }
    }

    fn get_postgresql_addr(&self, i: usize) -> Option<String> {
        match self.0.get(i) {
            Some(ref node) => Some(format!(
                "postgresql://{}:{}@{}",
                "postgres", "postgres", node.addr
            )),
            None => None,
        }
    }
}

impl Cluster<PostgresNode> for PostgresCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].clone()
    }
    fn get_cluster_node(&self, id: usize) -> PostgresNode {
        PostgresNode::new(self.get_node(id), self)
    }
    fn setup_test(&mut self, p: &HistParams) {
        self.create_variables(p.get_n_variable());

        let progress = self.1.clone();
        spawn(move || progress.join());
    }
    fn cleanup(&self) {
        self.drop_database();
    }
    fn info(&self) -> String {
        "PostgreSQL".to_string()
    }
}

fn main() {
    let matches = App::new("PostgreSQL")
        .version("1.0")
        .author("Ranadeep")
        .about("executes histories on PostgreSQL")
        .arg(
            Arg::with_name("hist_dir")
                .long("dir")
                .short("d")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("hist_out")
                .long("out")
                .short("o")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ip:port")
                .help("DB addr")
                .required(true),
        )
        .get_matches();

    let hist_dir = Path::new(matches.value_of("hist_dir").unwrap());
    let hist_out = Path::new(matches.value_of("hist_out").unwrap());

    fs::create_dir_all(hist_out).expect("couldn't create directory");

    let ips: Vec<_> = matches.values_of("ip:port").unwrap().collect();

    let mut cluster = PostgresCluster::new(&ips);

    cluster.execute_all(hist_dir, hist_out, 100);
}
