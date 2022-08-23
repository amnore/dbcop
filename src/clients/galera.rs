use crate::db::cluster::{Cluster, ClusterNode, Node};
use crate::db::history::{HistParams, Transaction};

use mysql::{Conn, TxOpts, prelude::*};

#[derive(Debug)]
pub struct GaleraNode {
    addr: String,
}

impl From<Node> for GaleraNode {
    fn from(node: Node) -> Self {
        GaleraNode {
            addr: format!("mysql://{}@{}", "root", node.addr),
        }
    }
}

impl ClusterNode for GaleraNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>) {
        let mut conn = Conn::new(self.addr.as_str()).unwrap();
        let txnopts = TxOpts::default()
            .set_isolation_level(Some(mysql::IsolationLevel::ReadCommitted))
            .set_access_mode(Some(mysql::AccessMode::ReadWrite))
            .set_with_consistent_snapshot(true);
        let read_stmt = conn.prep("SELECT * FROM dbcop.variables WHERE var=?").unwrap();
        let write_stmt = conn.prep("UPDATE dbcop.variables SET val=? WHERE var=?").unwrap();

        for transaction in hist.iter_mut() {
            while !transaction.success {
                transaction.success = true;
                let mut sqltxn = conn.start_transaction(txnopts).unwrap();

                for event in transaction.events.iter_mut() {
                    if event.write {
                        if let Err(_) = sqltxn.exec_drop(&write_stmt, (event.value, event.variable)) {
                            transaction.success = false;
                            break;
                        }
                        event.success = true;
                    } else {
                        match sqltxn.exec_first(&read_stmt, (event.variable,)) {
                            Ok(result) => {
                                let mut row: mysql::Row = result.unwrap();
                                event.value = row.take("val").unwrap();
                                event.success = true;
                            },
                            Err(_) => {
                                transaction.success = false;
                                break;
                            }
                        }
                    }
                }

                transaction.success = transaction.success && sqltxn.commit().is_ok();
            }
        }
    }
}

#[derive(Debug)]
pub struct GaleraCluster(Vec<Node>);

impl GaleraCluster {
    pub fn new(ips: &Vec<&str>) -> Self {
        GaleraCluster(GaleraCluster::node_vec(ips))
    }

    fn create_table(&self) -> bool {
        let addr = self.get_mysql_addr(0).unwrap();
        let mut conn = mysql::Conn::new(addr.as_str()).unwrap();

        conn.exec_drop("CREATE DATABASE IF NOT EXISTS dbcop", ()).unwrap();
        conn.exec_drop("DROP TABLE IF EXISTS dbcop.variables", ()).unwrap();
        conn.exec_drop(
            "CREATE TABLE IF NOT EXISTS dbcop.variables (var BIGINT(64) UNSIGNED NOT NULL PRIMARY KEY, val BIGINT(64) UNSIGNED NOT NULL)", ()
        ).unwrap();
        true
    }

    fn create_variables(&self, n_variable: usize) {
        let addr = self.get_mysql_addr(0).unwrap();
        let mut conn = mysql::Conn::new(addr.as_str()).unwrap();

        conn.exec_batch(
            "INSERT INTO dbcop.variables (var, val) values (?, 0)",
            (0..n_variable).map(|v| (v,))
        ).unwrap();
    }

    fn drop_database(&self) {
        let addr = self.get_mysql_addr(0).unwrap();
        let mut conn = mysql::Conn::new(addr.as_str()).unwrap();

        conn.exec_drop("DROP DATABASE dbcop", ()).unwrap();
    }

    fn get_mysql_addr(&self, i: usize) -> Option<String> {
        match self.0.get(i) {
            Some(ref node) => Some(format!("mysql://{}@{}", "root", node.addr)),
            None => None,
        }
    }
}

impl Cluster<GaleraNode> for GaleraCluster {
    fn n_node(&self) -> usize {
        self.0.len()
    }
    fn setup(&self) -> bool {
        self.create_table()
    }
    fn get_node(&self, id: usize) -> Node {
        self.0[id].clone()
    }
    fn get_cluster_node(&self, id: usize) -> GaleraNode {
        From::from(self.get_node(id))
    }
    fn setup_test(&mut self, p: &HistParams) {
        self.create_variables(p.get_n_variable());
    }
    fn cleanup(&self) {
        self.drop_database();
    }
    fn info(&self) -> String {
        "Galera".to_string()
    }
}
