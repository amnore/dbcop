use std::collections::{HashMap, HashSet};

use consistency::ser::Chains;

use consistency::util::EdgeClosure;

use petgraph::algo::astar;
// use petgraph::dot::{Config, Dot};
use petgraph::graph::node_index;
use petgraph::Graph;

use slog::Logger;

pub struct Causal {
    chains: Chains,
    co_closure: EdgeClosure,
    pub vis_pg: Graph<usize, usize>,
    pub co_pg: Graph<usize, usize>,
    log: Logger,
}

impl Causal {
    pub fn new(
        n_sizes: &Vec<usize>,
        txns_info: &HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)>,
        log: Logger,
    ) -> Self {
        let mut chains = Chains::new(&n_sizes, &txns_info, log.clone());
        chains.preprocess_wr();
        Causal {
            chains: chains,
            co_closure: EdgeClosure::new(),
            vis_pg: Graph::new(),
            co_pg: Graph::new(),
            log,
        }
    }

    pub fn preprocess_vis(&mut self) -> bool {
        for po in self.chains.tuple_to_id.iter().skip(1) {
            for &id in po.iter().rev().skip(1) {
                if self.chains.vis_closure.contains(id + 1, id) {
                    info!(self.log, "found cycle in WR");
                    return false;
                }
                self.chains.vis_closure.add_edge(id, id + 1);
                self.co_closure.add_edge(id, id + 1);

                self.vis_pg
                    .extend_with_edges(&[(id as u32, (id + 1) as u32, 1)]);
                self.co_pg
                    .extend_with_edges(&[(id as u32, (id + 1) as u32, 1)]);
            }
            if let Some(&u) = po.first() {
                if self.chains.vis_closure.contains(u, self.chains.root_txn_id) {
                    info!(self.log, "found cycle in WR");
                    return false;
                }
                self.chains.vis_closure.add_edge(self.chains.root_txn_id, u);
                self.co_closure.add_edge(self.chains.root_txn_id, u);

                self.vis_pg
                    .extend_with_edges(&[(self.chains.root_txn_id as u32, u as u32, 1)]);
                self.co_pg
                    .extend_with_edges(&[(self.chains.root_txn_id as u32, u as u32, 1)]);
            }
        }

        for (_, info) in self.chains.wr_order.iter() {
            for (&u, vs) in info {
                for &v in vs.iter() {
                    if self.chains.vis_closure.contains(v, u) {
                        info!(self.log, "found cycle in WR");
                        return false;
                    }
                    self.chains.vis_closure.add_edge(u, v);
                    self.co_closure.add_edge(u, v);

                    self.vis_pg.extend_with_edges(&[(u as u32, v as u32, 1)]);
                    self.co_pg.extend_with_edges(&[(u as u32, v as u32, 1)]);
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
                                //     info!(self.log,
                                //         "adding RW ({1}, {2}), WR_{3}({0}, {1}), {3} in W({2}), VIS({0}, {2})",
                                //         u, v, u_, _x
                                //     );
                                //     if self.chains.vis_closure.contains(u_, v) {
                                //         // info!(self.log,"cycle: {0} -> {1} -> {0}", v, u_);
                                //         info!(self.log,
                                //             "VIS*-RW cycle: {0:?} -> {1:?} -> {0:?}",
                                //             self.id_to_tuple[v], self.id_to_tuple[u_]
                                //         );
                                //         return false;
                                //     }
                                //     new_rw_edge.push((v, u_));
                                // }
                                if self.chains.vis_closure.contains(u_, v) {
                                    // info!(self.log,
                                    //     "adding WW ({2}, {0}), WR_{3}({0}, {1}), {3} in W({2}), VIS({2}, {1})",
                                    //     u, v, u_, _x
                                    // );

                                    if self.chains.vis_closure.contains(u, u_) {
                                        // info!(self.log,"cycle: {0} -> {1} -> {0}", u_, u);
                                        info!(
                                            self.log,
                                            "cycle: {0:?} co {1:?} vis {0:?}",
                                            self.chains.id_to_tuple[u_],
                                            self.chains.id_to_tuple[u]
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
                    // info!(self.log,"cycle: {0} -> {1} -> {0}", u, v);
                    info!(
                        self.log,
                        "cycle: {0:?} co {1:?} co {0:?}",
                        self.chains.id_to_tuple[u],
                        self.chains.id_to_tuple[v]
                    );

                    let co_path = astar(
                        &self.co_pg,
                        node_index(v),
                        |finish| finish == node_index(u),
                        |e| *e.weight(),
                        |_| 0,
                    );
                    info!(self.log, "{:?}", co_path);
                    if let Some((_, ref path)) = co_path {
                        for win2 in path.windows(2) {
                            let p_u = win2[0];
                            let p_v = win2[1];
                            if self.vis_pg.contains_edge(p_u, p_v) {
                                info!(
                                    self.log,
                                    "so/wr, {:?} {:?}",
                                    self.chains.id_to_tuple[p_u.index()],
                                    self.chains.id_to_tuple[p_v.index()]
                                );
                            } else {
                                info!(
                                    self.log,
                                    "co, {:?}, {:?}",
                                    self.chains.id_to_tuple[p_u.index()],
                                    self.chains.id_to_tuple[p_v.index()]
                                );
                                {
                                    let &(u, v, u_, _x) =
                                        ww_reason.get(&(p_u.index(), p_v.index())).unwrap();
                                    let vis_path = astar(
                                        &self.co_pg,
                                        node_index(u_),
                                        |finish| finish == node_index(v),
                                        |e| *e.weight(),
                                        |_| 0,
                                    )
                                    .unwrap()
                                    .1;
                                    info!(self.log, "reason:");
                                    info!(
                                        self.log,
                                        "vis path {:?} -> {:?}",
                                        self.chains.id_to_tuple[u_],
                                        self.chains.id_to_tuple[v]
                                    );
                                    let mut path_vec = Vec::new();
                                    for e in vis_path {
                                        path_vec.push(format!(
                                            "{:?}",
                                            self.chains.id_to_tuple[e.index()]
                                        ));
                                    }
                                    info!(self.log, "{:?}", path_vec);
                                    info!(
                                        self.log,
                                        "{:?} wr_{:?} {:?}",
                                        self.chains.id_to_tuple[u],
                                        _x,
                                        self.chains.id_to_tuple[v]
                                    );
                                }
                            }
                        }
                    }

                    info!(
                        self.log,
                        "co, {:?}, {:?}", self.chains.id_to_tuple[u], self.chains.id_to_tuple[v]
                    );
                    {
                        let &(u, v, u_, _x) = ww_reason.get(&(u, v)).unwrap();
                        let vis_path = astar(
                            &self.co_pg,
                            node_index(u_),
                            |finish| finish == node_index(v),
                            |e| *e.weight(),
                            |_| 0,
                        )
                        .unwrap()
                        .1;
                        info!(self.log, "reason:");
                        info!(
                            self.log,
                            "vis path {:?} -> {:?}",
                            self.chains.id_to_tuple[u_],
                            self.chains.id_to_tuple[v]
                        );
                        let mut path_vec = Vec::new();
                        for e in vis_path {
                            path_vec.push(format!("{:?} -> ", self.chains.id_to_tuple[e.index()]));
                        }
                        info!(self.log, "{:?}", path_vec);
                        info!(
                            self.log,
                            "{:?} wr_{:?} {:?}",
                            self.chains.id_to_tuple[u],
                            _x,
                            self.chains.id_to_tuple[v]
                        );
                    }

                    // info!(self.log,"{:?}", petgraph::dot::Dot::new(&self.co_pg));
                    return false;
                }
                is_converged &= !self.co_closure.add_edge(u, v);
                self.co_pg.extend_with_edges(&[(u as u32, v as u32, 1)]);
            }

            if is_converged {
                break;
            }
        }
        return true;
    }
}
