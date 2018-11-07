use db::history::{Event, Transaction};
use verifier::transactional_history_verify;

use std::collections::HashMap;

use rand;
use std::net::IpAddr;

use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use std::thread;

use std::convert::From;

use serde_yaml;

#[derive(Debug, Clone)]
pub struct Node {
    pub ip: IpAddr,
    pub id: usize,
}

pub trait ClusterNode {
    fn exec_session(&self, hist: &mut Vec<Transaction>);
}

#[derive(Clone, Debug, Default)]
pub struct TestParams {
    pub id: usize,
    pub n_variable: usize,
    pub n_transaction: usize,
    pub n_event: usize,
}

pub trait Cluster<N>
where
    N: 'static + Send + ClusterNode,
{
    fn n_node(&self) -> usize;
    fn setup(&self) -> bool;
    fn setup_test(&mut self, p: &TestParams);
    fn get_node(&self, id: usize) -> Node;
    fn get_cluster_node(&self, id: usize) -> N;
    fn cleanup(&self);

    fn node_vec(ips: &Vec<&str>) -> Vec<Node> {
        ips.iter()
            .enumerate()
            .map(|(i, ip)| Node {
                ip: ip.parse().unwrap(),
                id: i + 1,
            })
            .collect()
    }

    fn gen_history(&self, p: &TestParams) -> Vec<Vec<Transaction>> {
        let mut counters = HashMap::new();
        let mut random_generator = rand::thread_rng();
        let variable_range = Uniform::from(0..p.n_variable);
        let hist = (1..(self.n_node() + 1))
            .map(|i_node| {
                (0..p.n_transaction)
                    .map(|i_transaction| Transaction {
                        events: (0..p.n_event)
                            .map(|i_event| {
                                let variable = variable_range.sample(&mut random_generator);
                                let event = if random_generator.gen() {
                                    Event::read(variable)
                                } else {
                                    let value = {
                                        let entry = counters.entry(variable).or_insert(0);
                                        *entry += 1;
                                        *entry
                                    };
                                    Event::write(variable, value)
                                };
                                event
                            })
                            .collect(),
                        success: false,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        hist
    }

    fn test(&mut self, p: &TestParams) -> Option<usize> {
        // let mut hist = self.gen_history(p);
        use std::fs::File;
        use std::io::Read;

        let mut bytes = Vec::new();
        let mut file = File::open("../non-cc.yml").unwrap();
        file.read_to_end(&mut bytes).unwrap();
        let mut hist: Vec<Vec<Transaction>> = serde_yaml::from_slice(&bytes).unwrap();
        // println!("{:?}", hist);
        self.setup_test(p);
        println!("executing history - START");
        self.exec_history(&mut hist);
        println!("executing history - END");
        for (i_sesion, session) in hist.iter().enumerate() {
            println!("node {}", i_sesion + 1);
            for transaction in session.iter() {
                println!("{:?}", transaction);
            }
            println!();
        }

        // println!("# yaml");
        // println!("{}", serde_yaml::to_string(&hist).unwrap());
        // println!();

        transactional_history_verify(&hist);
        self.cleanup();
        None
    }

    fn exec_history(&self, hist: &mut Vec<Vec<Transaction>>) {
        let mut threads = (0..self.n_node())
            .zip(hist.drain(..))
            .map(|(node_id, mut single_hist)| {
                let cluster_node = self.get_cluster_node(node_id);
                thread::spawn(move || {
                    cluster_node.exec_session(&mut single_hist);
                    single_hist
                })
            })
            .collect::<Vec<_>>();
        hist.extend(threads.drain(..).map(|t| t.join().unwrap()));
    }
}
