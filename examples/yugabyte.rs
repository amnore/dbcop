extern crate clap;
extern crate dbcop;
extern crate postgres;

extern crate rand;

use rand::Rng;

use std::fs;
use std::path::Path;

use dbcop::db::cluster::{Cluster, ClusterNode, Node};
use dbcop::db::history::{HistParams, Transaction};

use clap::{App, Arg};

use postgres::{Client, NoTls};

#[derive(Debug)]
pub struct CockroachNode {
    addr: String,
    id: usize,
}

impl From<Node> for CockroachNode {
    fn from(node: Node) -> Self {
        CockroachNode {
            addr: format!("postgresql://{}:{}@{}", "yugabyte", "yugabyte", node.addr),
            id: node.id,
        }
    }
}

impl ClusterNode for CockroachNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let mut rng = rand::thread_rng();
        match Client::connect(self.addr.as_str(), NoTls) {
            Ok(mut conn) => hist.iter_mut().for_each(|transaction| {
                match conn
                    .build_transaction()
                    .isolation_level(postgres::IsolationLevel::Serializable)
                    .start()
                {
                    Ok(mut sqltxn) => {
                        transaction.events.iter_mut().for_each(|event| {
                            if event.write {
                                match sqltxn.execute(
                                    "UPDATE dbcop.variables SET val=$1 WHERE var=$2",
                                    &[&(event.value as i64), &(event.variable as i64)],
                                ) {
                                    Ok(_) => event.success = true,
                                    Err(_e) => {
                                        assert_eq!(event.success, false);
                                        // println!("WRITE ERR -- {:?}", _e);
                                    }
                                }
                            } else {
                                match sqltxn.query(
                                    "SELECT * FROM dbcop.variables WHERE var=$1",
                                    &[&(event.variable as i64)],
                                ) {
                                    Ok(result) => {
                                        if !result.is_empty() {
                                            let row = result.get(0);
                                            let value: i64 = row.unwrap().get("val");
                                            event.value = value as usize;
                                            event.success = true;
                                        } else {
                                            // may be diverged
                                            assert_eq!(event.success, false);
                                        }
                                    }
                                    Err(_e) => {
                                        // println!("READ ERR -- {:?}", _e);
                                        assert_eq!(event.success, false);
                                    }
                                }
                            }
                        });
                        match sqltxn.commit() {
                            Ok(_) => {
                                transaction.success = true;
                            }
                            Err(_e) => {
                                assert_eq!(transaction.success, false);
                                println!("{:?} -- COMMIT ERROR {}", transaction, _e);
                            }
                        }
                    }
                    Err(e) => println!("{:?} - TRANSACTION ERROR", e),
                }

                std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(100, 1000)));
            }),
            Err(_e) => {
                hist.iter().for_each(|transaction| {
                    assert_eq!(transaction.success, false);
                });
                // println!("CONNECTION ERROR {}", _e);}
            }
        }
    }
}

#[derive(Debug)]
pub struct CockroachCluster(Vec<Node>);

impl CockroachCluster {
    fn new(ips: &Vec<&str>) -> Self {
        CockroachCluster(CockroachCluster::node_vec(ips))
    }

    fn create_table(&self) -> bool {
        match self.get_postgresql_addr(0) {
            Some(ip) => Client::connect(ip.as_str(), NoTls)
                .and_then(|mut pool| {
                    pool.execute("CREATE DATABASE IF NOT EXISTS dbcop",  &[]).unwrap();
                    pool.execute("DROP TABLE IF EXISTS dbcop.variables",  &[]).unwrap();
                    pool.execute(
                        "CREATE TABLE IF NOT EXISTS dbcop.variables (var INT NOT NULL PRIMARY KEY, val INT NOT NULL)", &[]
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
                for stmt in conn
                    .prepare("INSERT INTO dbcop.variables (var, val) values ($1, 0)")
                    .into_iter()
                {
                    (0..n_variable).for_each(|variable| {
                        conn.execute(&stmt, &[&(variable as i64)])
                            .expect("Cannot create variable");
                    });
                }
            }
        }
    }

    fn drop_database(&self) {
        if let Some(ip) = self.get_postgresql_addr(0) {
            if let Ok(mut conn) = Client::connect(ip.as_str(), NoTls) {
                conn.execute("DROP DATABASE dbcop CASCADE", &[]).unwrap();
            }
        }
    }

    fn get_postgresql_addr(&self, i: usize) -> Option<String> {
        match self.0.get(i) {
            Some(ref node) => Some(format!("postgresql://{}@{}:26257", "root", node.addr)),
            None => None,
        }
    }
}

impl Cluster<CockroachNode> for CockroachCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].clone()
    }
    fn get_cluster_node(&self, id: usize) -> CockroachNode {
        From::from(self.get_node(id))
    }
    fn setup_test(&mut self, p: &HistParams) {
        self.create_variables(p.get_n_variable());
    }
    fn cleanup(&self) {
        self.drop_database();
    }
    fn info(&self) -> String {
        "CockroachDB".to_string()
    }
}

fn main() {
    let matches = App::new("CockroachDB")
        .version("1.0")
        .author("Ranadeep")
        .about("executes histories on CockroachDB")
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
                .help("Cluster addrs")
                .multiple(true)
                .required(true),
        )
        .get_matches();

    let hist_dir = Path::new(matches.value_of("hist_dir").unwrap());
    let hist_out = Path::new(matches.value_of("hist_out").unwrap());

    fs::create_dir_all(hist_out).expect("couldn't create directory");

    let ips: Vec<_> = matches.values_of("ip:port").unwrap().collect();

    let mut cluster = CockroachCluster::new(&ips);

    cluster.execute_all(hist_dir, hist_out, 100);
}
