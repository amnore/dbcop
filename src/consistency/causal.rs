use std::collections::{HashMap, HashSet};

use consistency::ser::Chains;

use consistency::util::EdgeClosure;

use petgraph::algo::astar;
use petgraph::dot::{Config, Dot};
use petgraph::graph::node_index;
use petgraph::Graph;

pub struct Causal {
    chains: Chains,
    co_closure: EdgeClosure,
    pub vis_PG: Graph<usize, usize>,
    pub co_PG: Graph<usize, usize>,
}

impl Causal {
    pub fn new(
        n_sizes: &Vec<usize>,
        txns_info: &HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)>,
    ) -> Self {
        let mut chains = Chains::new(&n_sizes, &txns_info);
        chains.preprocess_wr();
        Causal {
            chains: chains,
            co_closure: EdgeClosure::new(),
            vis_PG: Graph::new(),
            co_PG: Graph::new(),
        }
    }

    pub fn preprocess_vis(&mut self) -> bool {
        for po in self.chains.tuple_to_id.iter().skip(1) {
            for &id in po.iter().rev().skip(1) {
                if self.chains.vis_closure.contains(id + 1, id) {
                    println!("found cycle in WR");
                    return false;
                }
                self.chains.vis_closure.add_edge(id, id + 1);
                self.co_closure.add_edge(id, id + 1);

                self.vis_PG
                    .extend_with_edges(&[(id as u32, (id + 1) as u32, 1)]);
                self.co_PG
                    .extend_with_edges(&[(id as u32, (id + 1) as u32, 1)]);
            }
            if let Some(&u) = po.first() {
                if self.chains.vis_closure.contains(u, self.chains.root_txn_id) {
                    println!("found cycle in WR");
                    return false;
                }
                self.chains.vis_closure.add_edge(self.chains.root_txn_id, u);
                self.co_closure.add_edge(self.chains.root_txn_id, u);

                self.vis_PG
                    .extend_with_edges(&[(self.chains.root_txn_id as u32, u as u32, 1)]);
                self.co_PG
                    .extend_with_edges(&[(self.chains.root_txn_id as u32, u as u32, 1)]);
            }
        }

        for (_, info) in self.chains.wr_order.iter() {
            for (&u, vs) in info {
                for &v in vs.iter() {
                    if self.chains.vis_closure.contains(v, u) {
                        println!("found cycle in WR");
                        return false;
                    }
                    self.chains.vis_closure.add_edge(u, v);
                    self.co_closure.add_edge(u, v);

                    self.vis_PG.extend_with_edges(&[(u as u32, v as u32, 1)]);
                    self.co_PG.extend_with_edges(&[(u as u32, v as u32, 1)]);
                }
            }
        }

        return true;
    }

    pub fn preprocess_co(&mut self) -> bool {
        let mut ww_reason = HashMap::new();
        loop {
            // let mut new_rw_edge = Vec::new();
            let mut new_ww_edge = Vec::new();

            for (&_x, wr_x) in self.chains.wr_order.iter() {
                for (&u, vs) in wr_x.iter() {
                    for &v in vs.iter() {
                        for (&u_, _) in wr_x.iter() {
                            if u != u_ && v != u_ {
                                // if self.chains.vis_closure.contains(u, u_) {
                                //     println!(
                                //         "adding RW ({1}, {2}), WR_{3}({0}, {1}), {3} in W({2}), VIS({0}, {2})",
                                //         u, v, u_, _x
                                //     );
                                //     if self.chains.vis_closure.contains(u_, v) {
                                //         // println!("cycle: {0} -> {1} -> {0}", v, u_);
                                //         println!(
                                //             "VIS*-RW cycle: {0:?} -> {1:?} -> {0:?}",
                                //             self.id_to_tuple[v], self.id_to_tuple[u_]
                                //         );
                                //         return false;
                                //     }
                                //     new_rw_edge.push((v, u_));
                                // }
                                if self.chains.vis_closure.contains(u_, v) {
                                    // println!(
                                    //     "adding WW ({2}, {0}), WR_{3}({0}, {1}), {3} in W({2}), VIS({2}, {1})",
                                    //     u, v, u_, _x
                                    // );

                                    if self.chains.vis_closure.contains(u, u_) {
                                        // println!("cycle: {0} -> {1} -> {0}", u_, u);
                                        println!(
                                            "cycle: {0:?} co {1:?} vis {0:?}",
                                            self.chains.id_to_tuple[u_], self.chains.id_to_tuple[u]
                                        );
                                        return false;
                                    }
                                    new_ww_edge.push((u_, u));
                                    ww_reason.insert((u_, u), (u, v, u_, _x));
                                }
                            }
                        }
                    }
                }
            }

            let mut is_converged = true;

            for (u, v) in new_ww_edge {
                if self.co_closure.contains(v, u) {
                    // println!("cycle: {0} -> {1} -> {0}", u, v);
                    println!(
                        "cycle: {0:?} co {1:?} co {0:?}",
                        self.chains.id_to_tuple[u], self.chains.id_to_tuple[v]
                    );

                    let co_path = astar(
                        &self.co_PG,
                        node_index(v),
                        |finish| finish == node_index(u),
                        |e| *e.weight(),
                        |_| 0,
                    );
                    println!("{:?}", co_path);
                    if let Some((_, ref path)) = co_path {
                        for win2 in path.windows(2) {
                            let p_u = win2[0];
                            let p_v = win2[1];
                            if self.vis_PG.contains_edge(p_u, p_v) {
                                println!(
                                    "so/wr, {:?} {:?}",
                                    self.chains.id_to_tuple[p_u.index()],
                                    self.chains.id_to_tuple[p_v.index()]
                                );
                            } else {
                                println!(
                                    "co, {:?}, {:?}",
                                    self.chains.id_to_tuple[p_u.index()],
                                    self.chains.id_to_tuple[p_v.index()]
                                );
                                {
                                    let &(u, v, u_, _x) =
                                        ww_reason.get(&(p_u.index(), p_v.index())).unwrap();
                                    let vis_path = astar(
                                        &self.co_PG,
                                        node_index(u_),
                                        |finish| finish == node_index(v),
                                        |e| *e.weight(),
                                        |_| 0,
                                    )
                                    .unwrap()
                                    .1;
                                    println!("reason:");
                                    println!(
                                        "vis path {:?} -> {:?}",
                                        self.chains.id_to_tuple[u_], self.chains.id_to_tuple[v]
                                    );
                                    for e in vis_path {
                                        print!("{:?} -> ", self.chains.id_to_tuple[e.index()]);
                                    }
                                    println!();
                                    println!(
                                        "{:?} wr_{:?} {:?}",
                                        self.chains.id_to_tuple[u], _x, self.chains.id_to_tuple[v]
                                    );
                                }
                            }
                        }
                    }

                    println!(
                        "co, {:?}, {:?}",
                        self.chains.id_to_tuple[u], self.chains.id_to_tuple[v]
                    );
                    {
                        let &(u, v, u_, _x) = ww_reason.get(&(u, v)).unwrap();
                        let vis_path = astar(
                            &self.co_PG,
                            node_index(u_),
                            |finish| finish == node_index(v),
                            |e| *e.weight(),
                            |_| 0,
                        )
                        .unwrap()
                        .1;
                        println!("reason:");
                        println!(
                            "vis path {:?} -> {:?}",
                            self.chains.id_to_tuple[u_], self.chains.id_to_tuple[v]
                        );
                        for e in vis_path {
                            print!("{:?} -> ", self.chains.id_to_tuple[e.index()]);
                        }
                        println!();
                        println!(
                            "{:?} wr_{:?} {:?}",
                            self.chains.id_to_tuple[u], _x, self.chains.id_to_tuple[v]
                        );
                    }

                    // println!("{:?}", petgraph::dot::Dot::new(&self.co_PG));
                    return false;
                }
                is_converged &= !self.co_closure.add_edge(u, v);
                self.co_PG.extend_with_edges(&[(u as u32, v as u32, 1)]);
            }

            if is_converged {
                break;
            }
        }
        return true;
    }
}
