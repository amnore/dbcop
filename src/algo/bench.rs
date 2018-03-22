extern crate mysql;

use db::slowq;
use db::op;
use algo::txn;

use std::thread;

pub fn do_bench(conn_str: String) {
    let pool = mysql::Pool::new(conn_str.clone()).unwrap();

    let n_vars = 1000;

    op::create_table(&pool);
    op::create_vars(n_vars, &pool);

    slowq::turn_on_slow_query(&pool);
    slowq::clean_slow_query(&pool);

    let mut threads = vec![];

    for i in 0..5 {
        let conn_str_ = conn_str.clone();
        threads.push(
            thread::Builder::new()
                .name(format!("thread-{}", i))
                .spawn(move || {
                    let n_txns = 10;
                    let n_evts = 10;
                    let mut txns = txn::create_txns(n_txns, n_vars, n_evts);

                    let loc_pool = mysql::Pool::new(conn_str_).unwrap();

                    println!(
                        "thread-{} using connection_id {}",
                        i,
                        op::get_connection_id(&loc_pool)
                    );

                    for ref mut txn in txns.iter_mut() {
                        op::do_transaction(txn, &loc_pool);
                    }
                })
                .unwrap(),
        );
    }

    for t in threads {
        t.join().expect("thread failed");
    }

    op::drop_database(&pool);
}
