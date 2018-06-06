extern crate pbr;

use self::pbr::ProgressBar;

use algo::txn;
use db::op;
use db::slowq;
use mysql;

use std::sync::{Arc, Mutex};
use std::thread;

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use std::net::Ipv4Addr;

fn reachable(root: u64, read_map: &HashMap<u64, HashMap<usize, u64>>) -> HashSet<u64> {
    let mut stack = Vec::new();
    let mut seen = HashSet::new();

    stack.push(root);
    // seen.insert(root);

    while let Some(u) = stack.pop() {
        if let Some(vs) = read_map.get(&u) {
            for &v in vs.values() {
                if seen.insert(v) {
                    stack.push(v);
                }
            }
        }
    }

    seen
}

fn is_irreflexive(read_map: &HashMap<u64, HashMap<usize, u64>>) -> bool {
    for &e in read_map.keys() {
        let r = reachable(e, &read_map);
        if r.contains(&e) {
            println!("found {} {:?}", e, r);
            return false;
        }
    }
    return true;
}

fn get_wr_map(execution: &Vec<txn::Transaction>) -> HashMap<usize, HashMap<usize, Vec<usize>>> {
    let mut write_val = HashMap::new();
    execution.iter().enumerate().for_each(|(txn_id, txn)| {
        if txn.commit {
            txn.events.iter().for_each(|ev| {
                if ev.is_write() {
                    write_val.insert(ev.var.clone(), txn_id + 1);
                }
            });
        }
    });
    let mut write_read_x_vec = HashMap::new();
    execution.iter().enumerate().for_each(|(txn_id, txn)| {
        if txn.commit {
            txn.events.iter().for_each(|ev| {
                if ev.is_read() {
                    let write_txn = if ev.var.val == 0 {
                        0
                    } else {
                        write_val[&ev.var]
                    };
                    let var_entry = write_read_x_vec.entry(ev.var.id).or_insert(HashMap::new());
                    let write_entry = var_entry.entry(write_txn).or_insert(Vec::new());
                    write_entry.push(txn_id);
                }
            });
        }
    });
    write_read_x_vec
}

fn get_ww_order(execution: &Vec<txn::Transaction>) -> HashMap<usize, Vec<usize>> {
    let mut write_x_map: HashMap<usize, HashSet<usize>> = HashMap::new();
    execution.iter().enumerate().for_each(|(txn_id, txn)| {
        if txn.commit {
            txn.events.iter().for_each(|ev| {
                if ev.is_write() {
                    let var_id = ev.var.id;
                    let entry = write_x_map.entry(var_id).or_insert(HashSet::new());
                    entry.insert(txn_id + 1);
                }
            });
        }
    });
    HashMap::from_iter(write_x_map.into_iter().map(|(var, txn_ids)| {
        let mut vec = Vec::from_iter(txn_ids.into_iter());
        vec.sort_by(|&a, &b| {
            execution[a - 1]
                .end
                .query_time
                .cmp(&execution[b - 1].end.query_time)
        });
        (var, vec)
    }))
}

pub fn do_single_node(node: usize, vars: &Vec<usize>) {
    let mysql_addr = format!(
        "mysql://{}@{}",
        "root",
        Ipv4Addr::new(172, 18, 0, 11 + node)
    );

    let mut conn = mysql::Pool::new(mysql_addr).unwrap().get_conn().unwrap();

    let mut rng = rand::thread_rng();

    let mut v = Vec::new();

    for wr_txn in 0..n_txn {
        for wr_pos in 0..n_evts_per_txn {
            if rng.gen() {
                // do read
                v.push(Event::read(Variable::new(id, 0)));
            } else {
                // do write
                counters[id] += 1;
                v.push(Event::write(Variable::new(id, counters[id])));
            }
        }
    }
}

pub fn single_bench(nodes: &Vec<String>, vars: &Vec<usize>) {
    let n_vars = 5;
    let n_txn = 6;
    let n_evts_per_txn = 4;
    let n_iter = 100;

    {
        let mut conn = mysql::Pool::new(nodes[0].clone())
            .unwrap()
            .get_conn()
            .unwrap();

        op::create_vars(vars, &mut conn);
    }

    for nodes in 0..6 {
        do_single_node()
    }
}

pub fn do_bench() {
    let n_vars = 5;
    let n_txn = 6;
    let n_evts_per_txn = 4;
    let n_iter = 100;

    {
        // let mut tc = mysql::Pool::new(conn_str.clone())
        //     .unwrap()
        //     .get_conn()
        //     .unwrap();
        // slowq::increase_max_connections(1000000, &mut tc);
        // slowq::turn_on_slow_query(&mut tc);
    }

    let mut nodes = Vec::with_capacity(6);
    for i in 0..6 {
        nodes.push(format!(
            "mysql://{}@{}",
            "root",
            Ipv4Addr::new(172, 18, 0, 11 + i)
        ));
    }

    let threads = Vec::new();

    let session_histories = Vec::with_capacity(6);

    for i in 0..6 {
        session_histories.push(Arc::new(Mutex::new(Vec::new())));
    }

    for i in 0..6 {
        println!("{}", n);

        let history = session_histories[i].clone();
        let node_addr = nodes[i].clone();

        threads.push(thread::spawn(move || {
            let mut loc_conn = curr_conn.lock().unwrap();
            let mut loc_txn = curr_txn.lock().unwrap();
            op::do_transaction(&mut loc_txn, &mut loc_conn);
        }));
    }

    {
        let mut conn = mysql::Pool::new(nodes[0].clone())
            .unwrap()
            .get_conn()
            .unwrap();

        op::create_table(&mut conn);
    }

    //
    // let mut conns = vec![];
    // let mut conn_ids = vec![];
    // let mut executed_txns = vec![];
    //
    // let txns = {
    //     let mut txns_ = vec![];
    //     let mut counters = vec![0; n_vars + 1];
    //
    //     for _ in 0..n_txn {
    //         txns_.push(Arc::new(Mutex::new(txn::create_txn(
    //             n_vars,
    //             n_evts_per_txn,
    //             &mut counters,
    //         ))));
    //     }
    //
    //     txns_
    // };
    //
    // for _ in 0..n_txn {
    //     let mut txn_conn = mysql::Pool::new(&conn_str).unwrap().get_conn().unwrap();
    //     conn_ids.push(op::get_connection_id(&mut txn_conn));
    //     conns.push(Arc::new(Mutex::new(txn_conn)));
    // }
    //
    // let mut pb = ProgressBar::new(n_iter);
    // pb.format("╢▌▌░╟");
    //
    // for _ in 0..n_iter {
    //     op::clean_table(&mut conn);
    //     slowq::clean_slow_query(&mut conn);
    //     let mut threads = vec![];
    //     for i in 0..n_txn {
    //         let curr_txn = txns[i].clone();
    //         let curr_conn = conns[i].clone();
    //         threads.push(thread::spawn(move || {
    //             let mut loc_conn = curr_conn.lock().unwrap();
    //             let mut loc_txn = curr_txn.lock().unwrap();
    //
    //             op::do_transaction(&mut loc_txn, &mut loc_conn);
    //         }));
    //     }
    //
    //     for t in threads {
    //         t.join().expect("thread failed");
    //     }
    //
    //     pb.inc();
    //
    //     let mut curr_txns = txns.iter()
    //         .map(|x| x.lock().unwrap().clone())
    //         .collect::<Vec<_>>();
    //
    //     for i in 0..n_txn {
    //         let conn_id = conn_ids[i];
    //         curr_txns[i].start = slowq::get_start_txn_durations(conn_id, &mut conn);
    //         curr_txns[i].end = slowq::get_end_txn_durations(conn_id, &mut conn);
    //         let mut access_durs = slowq::get_access_durations(conn_id, &mut conn);
    //         for j in 0..curr_txns[i].events.len() {
    //             curr_txns[i].events[j].dur = access_durs[j].clone();
    //         }
    //     }
    //     executed_txns.push(curr_txns);
    // }
    //
    // println!("\n\n");
    //
    // // TODO: use cpupool
    // executed_txns.iter().for_each(|each_execution| {
    //     let wr_map = get_wr_map(&each_execution);
    //     let ww_order = get_ww_order(&each_execution);
    //     println!("{:?} ||||| {:?}", wr_map, ww_order);
    // });

    // println!("{:?}", conn_ids);
    // println!("{:#?}", executed_txns.first().unwrap());

    // op::drop_database(&mut conn);
}
