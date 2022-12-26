use std::collections::{HashMap, HashSet};

use crate::verifier::{TransactionId, TransactionInfo};

use super::util::{ConstrainedLinearization, DiGraph};

type Variable = usize;

#[derive(Debug, Default)]
pub struct AtomicHistoryPO {
    pub so: DiGraph<TransactionId>,
    pub vis: DiGraph<TransactionId>,
    pub root: TransactionId,
    pub txns_info: HashMap<TransactionId, TransactionInfo>,
    pub wr_rel: HashMap<Variable, DiGraph<TransactionId>>,
}

impl AtomicHistoryPO {
    pub fn new(txns_info: HashMap<TransactionId, TransactionInfo>) -> AtomicHistoryPO {
        let root = TransactionId(0, 0);
        let mut so: DiGraph<TransactionId> = Default::default();

        {
            let mut transactions: Vec<_> = txns_info.keys().cloned().collect();
            transactions.sort_unstable();

            for ts in transactions.windows(2) {
                so.add_edge(if ts[0].0 == ts[1].0 { ts[0] } else { TransactionId(0, 0) }, ts[1])
            }
        }

        so.take_closure();

        let mut wr_rel: HashMap<Variable, DiGraph<TransactionId>> = Default::default();

        for (&txn_id, txn_info) in txns_info.iter() {
            for &var in txn_info.write.iter() {
                wr_rel
                    .entry(var)
                    .or_insert_with(Default::default)
                    .add_vertex(txn_id);
            }
            for (&var, &txn_id2) in txn_info.read_from.iter() {
                wr_rel
                    .entry(var)
                    .or_insert_with(Default::default)
                    .add_edge(txn_id2, txn_id);
            }
        }

        AtomicHistoryPO {
            vis: so.clone(),
            so,
            root,
            txns_info,
            wr_rel,
        }
    }

    pub fn get_wr(&self) -> DiGraph<TransactionId> {
        let mut wr: DiGraph<TransactionId> = Default::default();

        for (_, wr_x) in self.wr_rel.iter() {
            wr.union_with(wr_x);
        }

        wr
    }

    pub fn vis_includes(&mut self, g: &DiGraph<TransactionId>) -> bool {
        self.vis.union_with(g)
    }

    pub fn vis_is_trans(&mut self) -> bool {
        let closure = self.vis.take_closure();
        let change = self
            .vis
            .adj_map
            .iter()
            .any(|(k, v)| closure.adj_map.get(k).unwrap().difference(v).count() > 0);
        self.vis = closure;
        change
    }

    pub fn causal_ww(&mut self) -> HashMap<Variable, DiGraph<TransactionId>> {
        let mut ww: HashMap<Variable, DiGraph<TransactionId>> = Default::default();

        for (&x, wr_x) in self.wr_rel.iter() {
            let mut ww_x: DiGraph<TransactionId> = Default::default();
            for (t1, t3s) in wr_x.adj_map.iter() {
                for (t2, _) in wr_x.adj_map.iter() {
                    if t1 != t2
                        && (self.vis.has_edge(t2, t1)
                            || t3s.iter().any(|t3| self.vis.has_edge(t2, t3)))
                    {
                        ww_x.add_edge(*t2, *t1);
                    }
                }
            }
            ww.insert(x, ww_x);
        }

        ww
    }

    pub fn causal_rw(&mut self) -> HashMap<Variable, DiGraph<TransactionId>> {
        let mut rw: HashMap<Variable, DiGraph<TransactionId>> = Default::default();

        for (&x, wr_x) in self.wr_rel.iter() {
            let mut rw_x: DiGraph<TransactionId> = Default::default();
            for (t1, t3s) in wr_x.adj_map.iter() {
                for (t2, _) in wr_x.adj_map.iter() {
                    if t1 != t2 {
                        for t3 in t3s.iter() {
                            if t3 != t2 && (self.vis.has_edge(t3, t2) || self.vis.has_edge(t1, t2))
                            {
                                rw_x.add_edge(*t3, *t2);
                            }
                        }
                    }
                }
            }
            rw.insert(x, rw_x);
        }

        rw
    }
}

#[derive(Debug)]
pub struct PrefixConsistentHistory {
    pub history: AtomicHistoryPO,
    pub active_write: HashMap<Variable, HashSet<TransactionId>>,
}

impl PrefixConsistentHistory {
    pub fn new(txns_info: HashMap<TransactionId, TransactionInfo>) -> Self {
        Self {
            history: AtomicHistoryPO::new(txns_info),
            active_write: Default::default(),
        }
    }
}

impl ConstrainedLinearization for PrefixConsistentHistory {
    type Vertex = (TransactionId, bool);
    fn get_root(&self) -> Self::Vertex {
        (TransactionId(0, 0), false)
    }

    fn children_of(&self, u: &Self::Vertex) -> Option<Vec<Self::Vertex>> {
        if u.1 {
            self.history
                .vis
                .adj_map
                .get(&u.0)
                .map(|vs| vs.iter().map(|&v| (v, false)).collect())
        } else {
            Some(vec![(u.0, true)])
        }
    }

    fn forward_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(&curr_txn.0).unwrap();
        if curr_txn.1 {
            for &x in curr_txn_info.write.iter() {
                let read_by = self
                    .history
                    .wr_rel
                    .get(&x)
                    .unwrap()
                    .adj_map
                    .get(&curr_txn.0)
                    .unwrap();
                self.active_write.insert(x, read_by.clone());
            }
        } else {
            for (&x, _) in curr_txn_info.read_from.iter() {
                assert!(self
                    .active_write
                    .entry(x)
                    .or_insert_with(Default::default)
                    .remove(&curr_txn.0));
            }
        }
        self.active_write.retain(|_, ts| !ts.is_empty());
    }

    fn backtrack_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(&curr_txn.0).unwrap();
        if curr_txn.1 {
            for &x in curr_txn_info.write.iter() {
                self.active_write.remove(&x);
            }
        } else {
            for (&x, _) in curr_txn_info.read_from.iter() {
                self.active_write
                    .entry(x)
                    .or_insert_with(Default::default)
                    .insert(curr_txn.0);
            }
        }
    }

    fn allow_next(&self, _linearization: &[Self::Vertex], v: &Self::Vertex) -> bool {
        if v.1 {
            let curr_txn_info = self.history.txns_info.get(&v.0).unwrap();
            curr_txn_info
                .write
                .iter()
                .all(|x| match self.active_write.get(x) {
                    Some(ts) if ts.len() == 1 => ts.iter().next().unwrap() == &v.0,
                    None => true,
                    _ => false,
                })
        } else {
            true
        }
    }

    fn vertices(&self) -> Vec<Self::Vertex> {
        self.history
            .txns_info
            .keys()
            .map(|&u| vec![(u, false), (u, true)])
            .flatten()
            .collect()
    }
}

#[derive(Debug)]
pub struct SnapshotIsolationHistory {
    pub history: AtomicHistoryPO,
    pub active_write: HashMap<Variable, HashSet<TransactionId>>,
    pub active_variable: HashSet<Variable>,
}

impl SnapshotIsolationHistory {
    pub fn new(txns_info: HashMap<TransactionId, TransactionInfo>) -> Self {
        Self {
            history: AtomicHistoryPO::new(txns_info),
            active_write: Default::default(),
            active_variable: Default::default(),
        }
    }
}

impl ConstrainedLinearization for SnapshotIsolationHistory {
    type Vertex = (TransactionId, bool);
    fn get_root(&self) -> Self::Vertex {
        (TransactionId(0, 0), false)
    }

    fn children_of(&self, u: &Self::Vertex) -> Option<Vec<Self::Vertex>> {
        if u.1 {
            self.history
                .vis
                .adj_map
                .get(&u.0)
                .map(|vs| vs.iter().map(|&v| (v, false)).collect())
        } else {
            Some(vec![(u.0, true)])
        }
    }

    fn forward_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(&curr_txn.0).unwrap();
        if curr_txn.1 {
            for &x in curr_txn_info.write.iter() {
                let read_by = self
                    .history
                    .wr_rel
                    .get(&x)
                    .unwrap()
                    .adj_map
                    .get(&curr_txn.0)
                    .unwrap();
                self.active_write.insert(x, read_by.clone());
            }

            self.active_variable = self
                .active_variable
                .difference(&curr_txn_info.write)
                .cloned()
                .collect();
        } else {
            for (&x, _) in curr_txn_info.read_from.iter() {
                assert!(self
                    .active_write
                    .entry(x)
                    .or_insert_with(Default::default)
                    .remove(&curr_txn.0));
            }
            self.active_write.retain(|_, ts| !ts.is_empty());

            self.active_variable = self
                .active_variable
                .union(&curr_txn_info.write)
                .cloned()
                .collect();
        }
    }

    fn backtrack_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(&curr_txn.0).unwrap();
        if curr_txn.1 {
            for &x in curr_txn_info.write.iter() {
                self.active_write.remove(&x);
            }
            self.active_variable = self
                .active_variable
                .union(&curr_txn_info.write)
                .cloned()
                .collect();
        } else {
            for (&x, _) in curr_txn_info.read_from.iter() {
                self.active_write
                    .entry(x)
                    .or_insert_with(Default::default)
                    .insert(curr_txn.0);
            }
            self.active_variable = self
                .active_variable
                .difference(&curr_txn_info.write)
                .cloned()
                .collect();
        }
    }

    fn allow_next(&self, _linearization: &[Self::Vertex], v: &Self::Vertex) -> bool {
        if v.1 {
            let curr_txn_info = self.history.txns_info.get(&v.0).unwrap();
            curr_txn_info
                .write
                .iter()
                .all(|x| match self.active_write.get(x) {
                    Some(ts) if ts.len() == 1 => ts.iter().next().unwrap() == &v.0,
                    None => true,
                    _ => false,
                })
        } else {
            self.active_variable
                .intersection(&self.history.txns_info.get(&v.0).unwrap().write)
                .next()
                .is_none()
        }
    }

    fn vertices(&self) -> Vec<Self::Vertex> {
        self.history
            .txns_info
            .keys()
            .map(|&u| vec![(u, false), (u, true)])
            .flatten()
            .collect()
    }
}

#[derive(Debug)]
pub struct SerializableHistory {
    pub history: AtomicHistoryPO,
    pub active_write: HashMap<Variable, HashSet<TransactionId>>,
}

impl SerializableHistory {
    pub fn new(txns_info: HashMap<TransactionId, TransactionInfo>) -> Self {
        Self {
            history: AtomicHistoryPO::new(txns_info),
            active_write: Default::default(),
        }
    }
}

impl ConstrainedLinearization for SerializableHistory {
    type Vertex = TransactionId;
    fn get_root(&self) -> Self::Vertex {
        TransactionId(0, 0)
    }

    fn forward_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(curr_txn).unwrap();
        for (&x, _) in curr_txn_info.read_from.iter() {
            assert!(self
                .active_write
                .entry(x)
                .or_insert_with(Default::default)
                .remove(curr_txn));
        }
        for &x in curr_txn_info.write.iter() {
            let read_by = self
                .history
                .wr_rel
                .get(&x)
                .unwrap()
                .adj_map
                .get(curr_txn)
                .unwrap();
            self.active_write.insert(x, read_by.clone());
        }
        self.active_write.retain(|_, ts| !ts.is_empty());
    }

    fn backtrack_book_keeping(&mut self, linearization: &[Self::Vertex]) {
        let curr_txn = linearization.last().unwrap();
        let curr_txn_info = self.history.txns_info.get(curr_txn).unwrap();
        for &x in curr_txn_info.write.iter() {
            self.active_write.remove(&x);
        }
        for (&x, _) in curr_txn_info.read_from.iter() {
            self.active_write
                .entry(x)
                .or_insert_with(Default::default)
                .insert(*curr_txn);
        }
    }

    fn children_of(&self, u: &Self::Vertex) -> Option<Vec<Self::Vertex>> {
        self.history
            .vis
            .adj_map
            .get(u)
            .map(|vs| vs.iter().cloned().collect())
    }

    fn allow_next(&self, _linearization: &[Self::Vertex], v: &Self::Vertex) -> bool {
        let curr_txn_info = self.history.txns_info.get(v).unwrap();
        curr_txn_info
            .write
            .iter()
            .all(|x| match self.active_write.get(x) {
                Some(ts) if ts.len() == 1 => ts.iter().next().unwrap() == v,
                None => true,
                _ => false,
            })
    }

    fn vertices(&self) -> Vec<Self::Vertex> {
        self.history.txns_info.keys().cloned().collect()
    }
}
