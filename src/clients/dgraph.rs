use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::collections::HashMap;

use crate::db::cluster::{Cluster, ClusterNode, Node};
use crate::db::history::{HistParams, Transaction};

use clap::{App, Arg};

use dgraph_tonic::sync::{Client, Mutate, Query};
use dgraph_tonic::{Operation, Mutation, Response};
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct DGraphNode {
    addr: SocketAddr,
    id: usize,
}

#[derive(Serialize, Deserialize)]
struct KeyValuePair {
    uid: String,
    val: usize,
}

#[derive(Serialize, Deserialize)]
struct All {
    all: Vec<KeyValuePair>,
}

impl From<Response> for All {
    fn from(r: Response) -> Self {
        serde_json::from_slice(&r.json).unwrap()
    }
}

impl From<Node> for DGraphNode {
    fn from(node: Node) -> Self {
        DGraphNode {
            addr: node.addr,
            id: node.id,
        }
    }
}

impl ClusterNode for DGraphNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let client = Client::new(format!("http://{}", self.addr)).unwrap();

        for transaction in hist.iter_mut() {
            transaction.success = true;
            let mut txn = client.new_mutated_txn();

            for event in transaction.events.iter_mut() {
                if event.write {
                    let mut mu = Mutation::new();
                    mu.set_set_json(&KeyValuePair { uid: (event.variable + 1).to_string(), val: event.value }).expect("set_set_json");
                    txn.mutate(mu).unwrap();
                    event.success = true;
                } else {
                    let query = r#"
query all($a: int) {
    all(func: uid($a)) {
        uid,
        val
    }
}
"#;
                    let result = txn.query_with_vars(query, HashMap::from([("$a", (event.variable + 1).to_string())])).unwrap();
                    let all: All = result.try_into().unwrap();
                    event.value = all.all[0].val as usize;
                    event.success = true;
                }
            }

            transaction.success &= if let Err(e) = txn.commit() {
                // println!("{:?} -- COMMIT ERROR {}", transaction, e.root_cause());
                false
            } else {
                true
            };
        }
    }
}

#[derive(Debug)]
pub struct DGraphCluster(Vec<Node>);

impl DGraphCluster {
    pub fn new(ips: &Vec<&str>) -> Self {
        DGraphCluster(DGraphCluster::node_vec(ips))
    }

    fn create_table(&self) -> bool {
        let http_addr = format!("http://{}", self.get_dgraph_addr(0).unwrap());
        let client = Client::new(http_addr).unwrap();

        client.alter(Operation {
            drop_all: true,
            ..Default::default()
        }).expect("alter");

        client.alter(Operation {
            schema: r#"
val: int .

type KV {
  val
}
"#.to_string(),
            ..Default::default()
        }).expect("alter");

        true
    }

    fn create_variables(&self, n_variable: usize) {
        let client = Client::new(format!("http://{}", self.get_dgraph_addr(0).unwrap())).unwrap();
        let mut txn = client.new_mutated_txn();
        let data = All { all: (1..n_variable+1).map(|uid| KeyValuePair { uid: uid.to_string(), val: 0 }).collect() };
        let mut mu = Mutation::new();
        mu.set_set_json(&data).unwrap();
        txn.mutate(mu).unwrap();
        txn.commit().unwrap();
    }

    fn drop_database(&self) {
        let client = Client::new(format!("http://{}", self.get_dgraph_addr(0).unwrap())).unwrap();
        client.alter(Operation {
            drop_all: true,
            ..Default::default()
        }).expect("alter");
    }

    fn get_dgraph_addr(&self, i: usize) -> Option<SocketAddr> {
        self.0.get(i).map(|n| n.addr)
    }
}

impl Cluster<DGraphNode> for DGraphCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].clone()
    }
    fn get_cluster_node(&self, id: usize) -> DGraphNode {
        From::from(self.get_node(id))
    }
    fn setup_test(&mut self, p: &HistParams) {
        self.create_variables(p.get_n_variable());
    }
    fn cleanup(&self) {
        self.drop_database();
    }
    fn info(&self) -> String {
        "Dgraph".to_string()
    }
}

// fn main() {
//     let matches = App::new("DGraph")
//         .version("1.0")
//         .author("Ranadeep")
//         .about("executes histories on DGraph")
//         .arg(
//             Arg::with_name("hist_dir")
//                 .long("dir")
//                 .short("d")
//                 .takes_value(true)
//                 .required(true),
//         )
//         .arg(
//             Arg::with_name("hist_out")
//                 .long("out")
//                 .short("o")
//                 .takes_value(true)
//                 .required(true),
//         )
//         .arg(
//             Arg::with_name("ip:port")
//                 .help("DB addr")
//                 .required(true),
//         )
//         .get_matches();

//     let hist_dir = Path::new(matches.value_of("hist_dir").unwrap());
//     let hist_out = Path::new(matches.value_of("hist_out").unwrap());

//     fs::create_dir_all(hist_out).expect("couldn't create directory");

//     let ips: Vec<_> = matches.values_of("ip:port").unwrap().collect();

//     let mut cluster = DGraphCluster::new(&ips);

//     cluster.execute_all(hist_dir, hist_out, 100);
// }
