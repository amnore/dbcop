extern crate clap;
extern crate dbcop;
extern crate rsmgclient;
extern crate indicatif;

extern crate rand;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread::spawn;
use std::collections::HashMap;
use std::net::SocketAddr;

use dbcop::db::cluster::{Cluster, ClusterNode, Node};
use dbcop::db::history::{HistParams, Transaction};

use clap::{App, Arg};

use rsmgclient::{ConnectParams, Connection, SSLMode, QueryParam, ConnectionStatus};

use indicatif::{MultiProgress, ProgressBar};

#[derive(Debug)]
pub struct MemgraphNode {
    addr: SocketAddr,
    id: usize,
    progress: Arc<MultiProgress>,
}

impl MemgraphNode {
    fn new(node: Node, cluster: &MemgraphCluster) -> Self {
        MemgraphNode {
            addr: node.addr,
            id: node.id,
            progress: cluster.1.clone(),
        }
    }
}

impl ClusterNode for MemgraphNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let progress = self.progress.add(ProgressBar::new(hist.len() as u64));
        let reconnect = || {
            loop {
                if let Ok(conn) = Connection::connect(&ConnectParams {
                    address: Some(self.addr.ip().to_string()),
                    port: self.addr.port(),
                    sslmode: SSLMode::Disable,
                    lazy: false,
                    ..Default::default()
                }) {
                    return conn;
                }
            }
        };
        let mut conn = reconnect();

        for transaction in progress.wrap_iter(hist.iter_mut()) {
            loop {
                let mut success = true;
                for event in transaction.events.iter_mut() {
                    let params = HashMap::from([("var".to_string(), QueryParam::Int(event.variable as i64)), ("val".to_string(), QueryParam::Int(event.value as i64))]);
                    if event.write {
                        if let Err(e) = conn.execute("MATCH (n:KV {var: $var}) SET n.val = $val", Some(&params)) {
                            success = false;
                            // eprintln!("{}:{:?}:{:?}", line!(), e, conn.status());
                            break;
                        } 

                        if let Err(e) = conn.fetchall() {
                            success = false;
                            // eprintln!("{}:{:?}:{:?}:{:?}:{}", line!(), e, status, conn.status(), conn.lazy());
                            break;
                        }
                    } else {
                        if let Err(e) = conn.execute("MATCH (n:KV {var: $var}) RETURN n.val", Some(&params)) {
                            success = false;
                            // eprintln!("{}:{:?}", line!(), e);
                            break;
                        }

                        match conn.fetchall() {
                            Ok(rows) => {
                                event.success = true;
                                match rows[0].values[0] {
                                    rsmgclient::Value::Int(val) => event.value = val as usize,
                                    _ => unreachable!()
                                }
                            }
                            Err(e) => {
                                success = false;
                                // eprintln!("{}:{:?}", line!(), e);
                                break;
                            }
                        }
                    }
                }

                if !success {
                    assert!(conn.status() == ConnectionStatus::Bad);
                    reconnect();
                } else if conn.commit().is_ok() {
                    transaction.success = true;
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct MemgraphCluster(Vec<Node>, Arc<MultiProgress>);

impl MemgraphCluster {
    fn new(ips: &Vec<&str>) -> Self {
        MemgraphCluster(MemgraphCluster::node_vec(ips), Arc::new(MultiProgress::new()))
    }

    fn create_table(&self) -> bool {
        true
    }

    fn create_variables(&self, n_variable: usize) {
        let mut conn = self.get_memgraph_addr(0).as_ref()
            .and_then(|param| Connection::connect(param).ok()).unwrap();

        for i in 0..n_variable {
            conn.execute("CREATE (n:KV {var: $var, val: $val})",
                Some(&HashMap::from([("var".to_string(), QueryParam::Int(i as i64)), ("val".to_string(), QueryParam::Int(0))])))
                .unwrap();
            conn.fetchall().unwrap();
        }

        conn.commit().unwrap();
    }

    fn drop_database(&self) {
        let mut conn = self.get_memgraph_addr(0).as_ref()
            .and_then(|param| Connection::connect(param).ok()).unwrap();

        conn.execute_without_results("MATCH (n:KV) DELETE n").unwrap();
        conn.commit().unwrap();
    }

    fn get_memgraph_addr(&self, i: usize) -> Option<ConnectParams> {
        self.0.get(i).map(|node| ConnectParams {
            host: Some(node.addr.ip().to_string()),
            port: node.addr.port(),
            sslmode: SSLMode::Disable,
            ..Default::default()
        })
    }
}

impl Cluster<MemgraphNode> for MemgraphCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].clone()
    }
    fn get_cluster_node(&self, id: usize) -> MemgraphNode {
        MemgraphNode::new(self.get_node(id), self)
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
        "memgraph".to_string()
    }
}

fn main() {
    let matches = App::new("memgraph")
        .version("1.0")
        .author("Ranadeep")
        .about("executes histories on memgraph")
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

    let mut cluster = MemgraphCluster::new(&ips);

    cluster.execute_all(hist_dir, hist_out, 100);
}
