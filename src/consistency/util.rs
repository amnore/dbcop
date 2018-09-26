use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct EdgeClosure {
    pub forward_edge: HashMap<usize, HashSet<usize>>,
    pub backward_edge: HashMap<usize, HashSet<usize>>,
}

impl EdgeClosure {
    pub fn new() -> Self {
        EdgeClosure {
            forward_edge: HashMap::new(),
            backward_edge: HashMap::new(),
        }
    }

    pub fn contains(&self, u: usize, v: usize) -> bool {
        self.forward_edge
            .get(&u)
            .and_then(|vs| Some(vs.contains(&v)))
            == Some(true)
    }

    pub fn add_edge(&mut self, u: usize, v: usize) -> bool {
        // returns true if new edge added
        if !self.contains(u, v) {
            let mut new_edge = Vec::new();
            {
                let opt_prevs_u = self.backward_edge.get(&u);
                let opt_nexts_v = self.forward_edge.get(&v);
                if let Some(prevs_u) = opt_prevs_u {
                    if let Some(nexts_v) = opt_nexts_v {
                        for &prev_u in prevs_u.iter() {
                            for &next_v in nexts_v.iter() {
                                if !self.contains(prev_u, next_v) {
                                    new_edge.push((prev_u, next_v));
                                }
                            }
                        }
                    }
                }
                if let Some(prevs_u) = opt_prevs_u {
                    for &prev_u in prevs_u.iter() {
                        if !self.contains(prev_u, v) {
                            new_edge.push((prev_u, v));
                        }
                    }
                }
                if let Some(nexts_v) = opt_nexts_v {
                    for &next_v in nexts_v.iter() {
                        if !self.contains(u, next_v) {
                            new_edge.push((u, next_v));
                        }
                    }
                }
                new_edge.push((u, v));
            }
            for (u_, v_) in new_edge {
                let ent_u = self.forward_edge.entry(u_).or_insert_with(HashSet::new);
                ent_u.insert(v_);
                let ent_v = self.backward_edge.entry(v_).or_insert_with(HashSet::new);
                ent_v.insert(u_);
            }
            true
        } else {
            false
        }
    }
}
