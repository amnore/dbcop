use rand::{self, seq, Rng};

use algo::var::{Event, MySQLDur, Variable};

use std::fmt;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Transaction {
    pub commit: bool,
    pub events: Vec<Event>,
    pub start: MySQLDur,
    pub end: MySQLDur,
}

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            // "<{:?}, {}, {:?}, {:?}>",
            "<{:?}, {}, {:?}>",
            self.events,
            if self.commit { "COMMIT" } else { "ROLLBACK" },
            // self.start,
            self.end,
        )
    }
}

// impl Transaction {
//     pub fn is_acyclic_visibility(&self) -> bool {
//
//     }
// }

pub fn create_txn(n_var: usize, n_op: usize, counters: &mut Vec<usize>) -> Transaction {
    let mut rng = rand::thread_rng();
    let mut v = vec![];
    for id in seq::sample_iter(&mut rng, 1..n_var + 1, n_op).unwrap() {
        if rng.gen() {
            v.push(Event::read(Variable::new(id, 0)));
        } else {
            counters[id] += 1;
            v.push(Event::write(Variable::new(id, counters[id])));
        }
    }
    Transaction {
        events: v,
        // commit: rng.gen(),
        commit: true,
        start: MySQLDur::new(),
        end: MySQLDur::new(),
    }
}

pub fn create_txns(
    n_txn: usize,
    n_var: usize,
    n_op: usize,
    counters: &mut Vec<usize>,
) -> Vec<Transaction> {
    (0..n_txn)
        .map(|_| create_txn(n_var, n_op, counters))
        .collect()
}
