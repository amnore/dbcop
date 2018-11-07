extern crate antidotedb;
extern crate byteorder;
extern crate clap;
extern crate dbcop;

use dbcop::db::cluster::{Cluster, ClusterNode, Node, TestParams};
use dbcop::db::history::Transaction;

use clap::{App, Arg};

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;

use antidotedb::crdt::{Operation, LWWREG};
use antidotedb::AntidoteDB;

#[derive(Debug, Clone)]
pub struct AntidoteNode {
    node: Node,
    addr: String,
    id: usize,
    timestamp: Option<Vec<u8>>,
}

impl From<Node> for AntidoteNode {
    fn from(node: Node) -> Self {
        AntidoteNode {
            node: node.clone(),
            addr: format!("{}:8087", node.ip),
            id: node.id,
            timestamp: None,
        }
    }
}

impl ClusterNode for AntidoteNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let mut conn = AntidoteDB::connect_with_string(&self.addr);

        let mut timestamp = self.timestamp.clone();

        // println!("{:?}", timestamp);

        hist.iter_mut().for_each(|transaction| {
            let db_transaction = conn.start_transaction(timestamp.as_ref());

            transaction.events.iter_mut().for_each(|event| {
                let obj = LWWREG::new(&format!("{}", event.variable), "dbcop");
                if event.write {
                    let op = obj.set(event.value as u64);

                    match conn.mult_update_in_transaction(&[op], &db_transaction) {
                        Ok(_) => event.success = true,
                        Err(_e) => {
                            assert_eq!(event.success, false);
                            // println!("WRITE ERR -- {:?}", _e);
                        }
                    }
                } else {
                    match conn.mult_read_in_transaction(&[obj.clone()], &db_transaction) {
                        Ok(values) => {
                            let bytes = values[0].get_reg().get_value();
                            event.value =
                                Cursor::new(bytes).read_u64::<BigEndian>().unwrap() as usize;
                            event.success = true;
                        }
                        Err(_) => assert!(!event.success),
                    }
                }
            });

            match conn.commit_transaction(&db_transaction) {
                Ok(commit_time) => {
                    transaction.success = true;
                    timestamp = Some(commit_time);
                }
                Err(_e) => {
                    assert_eq!(transaction.success, false);
                    println!("{:?} -- COMMIT ERROR", transaction);
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct AntidoteCluster(Vec<AntidoteNode>);

impl AntidoteCluster {
    fn new(ips: &Vec<&str>) -> Self {
        let mut v = AntidoteCluster::node_vec(ips);
        let k: Vec<_> = v.drain(..).map(|x| From::from(x)).collect();
        AntidoteCluster(k)
    }

    fn create_table(&self) -> bool {
        true
    }

    fn create_variables(&mut self, n_variable: usize) {
        let mut conn = AntidoteDB::connect_with_string(&self.get_antidote_addr(0).unwrap());

        let db_transaction = conn.start_transaction(None);

        let ops: Vec<_> = (0..n_variable)
            .map(|variable| LWWREG::new(&format!("{}", variable), "dbcop").set(0))
            .collect();

        conn.mult_update_in_transaction(&ops, &db_transaction)
            .expect("error to init zero values");

        match conn.commit_transaction(&db_transaction) {
            Ok(commit_time) => {
                self.0.iter_mut().for_each(|x| {
                    x.timestamp = Some(commit_time.clone());
                });
            }
            Err(_e) => {
                println!("COMMIT ERROR while init");
            }
        }
    }

    fn drop_database(&self) {}

    fn get_antidote_addr(&self, i: usize) -> Option<String> {
        self.0.get(i).map(|ref node| node.addr.clone())
    }
}

impl Cluster<AntidoteNode> for AntidoteCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].node.clone()
    }
    fn get_cluster_node(&self, id: usize) -> AntidoteNode {
        self.0[id].clone()
    }
    fn setup_test(&mut self, p: &TestParams) {
        self.create_variables(p.n_variable);
    }
    fn cleanup(&self) {
        self.drop_database();
    }
}

fn main() {
    let matches = App::new("Antidote")
        .version("1.0")
        .author("Ranadeep")
        .about("verifies a Antidote cluster")
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
        .arg(Arg::with_name("history_output").long("output").short("o"))
        .arg(
            Arg::with_name("ips")
                .help("Cluster ips")
                .multiple(true)
                .required(true),
        )
        .get_matches();
    let ips: Vec<_> = matches.values_of("ips").unwrap().collect();

    let mut cluster = AntidoteCluster::new(&ips);

    // println!("{:?}", cluster);

    cluster.setup();

    // test_id, n_variable, n_transaction, n_event
    let params = TestParams {
        n_variable: matches.value_of("n_variable").unwrap().parse().unwrap(),
        n_transaction: matches.value_of("n_transaction").unwrap().parse().unwrap(),
        n_event: matches.value_of("n_event").unwrap().parse().unwrap(),
        ..Default::default()
    };

    println!("{:?}", params);

    cluster.test(&params);
}
