extern crate rand;

use self::rand::Rng;

use algo::var::{Event, Transaction, Variable};

pub fn create_txn(lim: u64, n_op: usize) -> Transaction {
    let mut rng = rand::thread_rng();
    let mut v = vec![];
    for _ in 0..n_op {
        if rng.gen() {
            let id = rng.gen::<u64>() % lim + 1;
            v.push(Event::read(Variable::new(id, 0)));
        } else {
            let id = rng.gen::<u64>() % lim + 1;
            let val = rng.gen::<u64>() % lim + 1;
            v.push(Event::write(Variable::new(id, val)));
        }
    }
    Transaction {
        events: v,
        commit: rng.gen(),
    }
}

pub fn create_txns(n: usize, lim: u64, n_op: usize) -> Vec<Transaction> {
    (0..n).map(|_| create_txn(lim, n_op)).collect()
}
