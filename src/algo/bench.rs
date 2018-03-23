extern crate mysql;

use db::slowq;
use db::op;
use algo::txn;

use std::thread;
use std::sync::{Arc, Mutex};

pub fn do_bench(conn_str: String) {
    let mut conn = mysql::Pool::new(conn_str.clone())
        .unwrap()
        .get_conn()
        .unwrap();

    let n_vars = 1000;
    let n_txn = 10;
    let n_evts_per_txn = 10;

    op::create_table(&mut conn);
    op::create_vars(n_vars, &mut conn);

    slowq::turn_on_slow_query(&mut conn);
    slowq::clean_slow_query(&mut conn);
    slowq::increase_max_connections(100000, &mut conn);

    let mut threads = vec![];

    let mut txns = vec![];
    let mut conn_ids = vec![];

    for _ in 0..n_txn {
        txns.push(Arc::new(Mutex::new(txn::create_txn(
            n_vars,
            n_evts_per_txn,
        ))));
        conn_ids.push(Arc::new(Mutex::new(0)));
    }

    for i in 0..n_txn {
        let conn_str_ = conn_str.clone();
        let mut curr_txn = txns[i].clone();
        let mut curr_conn_id = conn_ids[i].clone();
        threads.push(thread::spawn(move || {
            let mut loc_conn = mysql::Pool::new(conn_str_).unwrap().get_conn().unwrap();

            let mut curr_conn_id_ = curr_conn_id.lock().unwrap();
            *curr_conn_id_ = op::get_connection_id(&mut loc_conn);

            let mut curr_txn_ = curr_txn.lock().unwrap();
            op::do_transaction(&mut curr_txn_, &mut loc_conn);
        }));
    }

    for t in threads {
        t.join().expect("thread failed");
    }

    for i in 0..n_txn {
        println!(
            "Connection id: {}\n{:?}\n",
            *conn_ids[i].lock().unwrap(),
            *txns[i].lock().unwrap()
        )
    }

    // op::drop_database(&mut conn);
}
