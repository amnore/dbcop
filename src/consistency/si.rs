use std::collections::{HashMap, HashSet};

use consistency::ser::Chains;

use slog::Logger;

pub struct SIChains {}

impl SIChains {
    pub fn transform(
        // n_sizes: &Vec<usize>,
        txns_info: &HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)>,
    ) -> HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)> {
        let mut new_txns_info = HashMap::new();
        let mut curr_var = 0;
        for (&(po_id, txn_id), (rd_info, wr_info)) in txns_info.iter() {
            {
                let mut new_rd_info = HashMap::new();
                for (&x, &(wr_po_id, wr_txn_id)) in rd_info.iter() {
                    if wr_po_id == 0 {
                        assert_eq!(wr_txn_id, 0);
                        new_rd_info.insert(x, (wr_po_id, wr_txn_id));
                    } else {
                        new_rd_info.insert(x, (wr_po_id, (wr_txn_id << 1) + 1));
                    }
                }
                new_txns_info.insert((po_id, txn_id << 1), (new_rd_info, HashSet::new()));
            }
            {
                let new_wr_info = wr_info.clone();
                if let Some(&max_var) = wr_info.iter().max() {
                    if max_var > curr_var {
                        curr_var = max_var;
                    }
                }
                new_txns_info.insert((po_id, (txn_id << 1) + 1), (HashMap::new(), new_wr_info));
            }
        }
        // SI - CONFLICT axiom - WW < VIS, in Prefix Consistency, don't need to do this.
        for (&(po_id_u, txn_id_u), (_, wr_info_u)) in txns_info.iter() {
            for (&(po_id_v, txn_id_v), (_, wr_info_v)) in txns_info.iter() {
                if po_id_u != po_id_v {
                    if wr_info_u.intersection(&wr_info_v).next().is_some() {
                        curr_var += 1;
                        new_txns_info
                            .get_mut(&(po_id_u, txn_id_u << 1))
                            .unwrap()
                            .1
                            .insert(curr_var);
                        new_txns_info
                            .get_mut(&(po_id_u, (txn_id_u << 1) + 1))
                            .unwrap()
                            .0
                            .insert(curr_var, (po_id_u, txn_id_u << 1));
                        new_txns_info
                            .get_mut(&(po_id_v, (txn_id_v << 1) + 1))
                            .unwrap()
                            .1
                            .insert(curr_var);
                    }
                }
            }
        }
        // println!("{:?}", new_txns_info);
        new_txns_info
    }
    pub fn new(
        n_sizes: &Vec<usize>,
        txns_info: &HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)>,
        log: Logger,
    ) -> Chains {
        let new_n_sizes: Vec<_> = n_sizes.iter().map(|&x| x << 1).collect();
        let new_txns_info = SIChains::transform(txns_info);
        Chains::new(&new_n_sizes, &new_txns_info, log)
    }
}
