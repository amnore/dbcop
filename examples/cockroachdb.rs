extern crate clap;
extern crate dbcop;
extern crate postgres;

use dbcop::db::cluster::{Cluster, ClusterNode, Node, TestParams};
use dbcop::db::history::Transaction;

use clap::{App, Arg};

use postgres::{transaction, Connection, TlsMode};

#[derive(Debug)]
pub struct CockroachNode {
    addr: String,
    id: usize,
}

impl From<Node> for CockroachNode {
    fn from(node: Node) -> Self {
        CockroachNode {
            addr: format!("postgresql://{}@{}:26257", "root", node.ip),
            id: node.id,
        }
    }
}

impl ClusterNode for CockroachNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        match Connection::connect(self.addr.clone(), TlsMode::None) {
            Ok(conn) => hist.iter_mut().for_each(|transaction| {
                let mut config = transaction::Config::new();
                config.isolation_level(transaction::IsolationLevel::Serializable);
                match conn.transaction_with(&config) {
                    Ok(sqltxn) => {
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
                                            let mut row = result.get(0);
                                            let value : i64 = row.get("val");
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
            Some(ip) => Connection::connect(ip, TlsMode::None)
                .and_then(|pool| {
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
            if let Ok(conn) = Connection::connect(ip, TlsMode::None) {
                for mut stmt in conn
                    .prepare("INSERT INTO dbcop.variables (var, val) values ($1, 0)")
                    .into_iter()
                {
                    (0..n_variable).for_each(|variable| {
                        stmt.execute(&[&(variable as i64)]).unwrap();
                    });
                }
            }
        }
    }

    fn drop_database(&self) {
        if let Some(ip) = self.get_postgresql_addr(0) {
            if let Ok(conn) = Connection::connect(ip, TlsMode::None) {
                conn.execute("DROP DATABASE dbcop CASCADE", &[]).unwrap();
            }
        }
    }

    fn get_postgresql_addr(&self, i: usize) -> Option<String> {
        match self.0.get(i) {
            Some(ref node) => Some(format!("postgresql://{}@{}:26257", "root", node.ip)),
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
    fn setup_test(&mut self, p: &TestParams) {
        self.create_variables(p.n_variable);
    }
    fn cleanup(&self) {
        self.drop_database();
    }
}

fn main() {
    let matches = App::new("Cockroach")
        .version("1.0")
        .author("Ranadeep")
        .about("verifies a Cockroach cluster")
        .arg(
            Arg::with_name("n_variable")
                .long("nval")
                .short("v")
                .default_value("5"),
        )
        .arg(
            Arg::with_name("n_transaction")
                .long("ntxn")
                .short("t")
                .default_value("5"),
        )
        .arg(
            Arg::with_name("n_event")
                .long("nevt")
                .short("e")
                .default_value("2"),
        )
        .arg(
            Arg::with_name("ips")
                .help("Cluster ips")
                .multiple(true)
                .required(true),
        )
        .get_matches();
    let ips: Vec<_> = matches.values_of("ips").unwrap().collect();

    let mut cluster = CockroachCluster::new(&ips);

    // println!("{:?}", cluster);

    cluster.setup();

    // test_id, n_variable, n_transaction, n_event
    let params = TestParams {
        id: 0,
        n_variable: matches.value_of("n_variable").unwrap().parse().unwrap(),
        n_transaction: matches.value_of("n_transaction").unwrap().parse().unwrap(),
        n_event: matches.value_of("n_event").unwrap().parse().unwrap(),
    };

    println!("{:?}", params);

    cluster.test(&params);
}
