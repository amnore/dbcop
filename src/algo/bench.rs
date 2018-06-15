// use self::pbr::ProgressBar;

// use algo::txn;
use db::op;
// use db::slowq;
use mysql;
use rand;

use rand::Rng;

use std::sync::{Arc, Mutex};
use std::thread;

use std::net::Ipv4Addr;

use consistency::ser::Chains;

use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub enum Event {
    READ,
    WRITE,
}

#[derive(Debug)]
pub struct Action {
    ev: Event,
    var: usize,
    wr_node: usize,
    wr_txn: usize,
    wr_pos: usize,
}

pub fn do_single_node(node: DBNode, vars: &Vec<usize>, history: &mut Vec<Vec<Action>>) {
    // println!("doing for {:?} with {:?}", node, vars);
    let n_txn = 5;
    let n_evts_per_txn = 3;
    // let n_iter = 100;

    match mysql::Pool::new(node.addr) {
        Ok(conn) => {
            let mut rng = rand::thread_rng();

            for wr_txn in 0..n_txn {
                let mut curr_txn = Vec::new();
                for mut sqltxn in conn.start_transaction(
                    false,
                    Some(mysql::IsolationLevel::Serializable),
                    Some(false),
                ) {
                    for wr_pos in 0..n_evts_per_txn {
                        let curr_var = *rng.choose(&vars).unwrap();
                        if rng.gen() {
                            match sqltxn.prep_exec(
                            "UPDATE dbcop.variables SET wr_node=?, wr_txn=?, wr_pos=? WHERE id=?",
                            (node.id, history.len(), curr_txn.len(), curr_var),
                        ) {
                            Err(_e) => {
                                println!("WRITE ERR {} {} {} {}-- {:?}", curr_var, node.id, wr_txn, wr_pos, _e);
                            }
                            _ => {
                                let act = Action {
                                    ev: Event::WRITE,
                                    var: curr_var,
                                    wr_node: node.id,
                                    wr_txn: history.len(),
                                    wr_pos: curr_txn.len(),
                                };
                                curr_txn.push(act);
                            }
                        }
                        } else {
                            match sqltxn
                                .prep_exec("SELECT * FROM dbcop.variables WHERE id=?", (curr_var,))
                                .and_then(|mut rows| {
                                    let mut row = rows.next().unwrap().unwrap();
                                    // assert_eq!(e.var.id, row.take::<u64, &str>("id").unwrap());
                                    let _id = row.take("id").unwrap();
                                    let _wr_node = row.take("wr_node").unwrap();
                                    let _wr_txn = row.take("wr_txn").unwrap();
                                    let _wr_pos = row.take("wr_pos").unwrap();

                                    let act = Action {
                                        ev: Event::READ,
                                        var: _id,
                                        wr_node: _wr_node,
                                        wr_txn: _wr_txn,
                                        wr_pos: _wr_pos,
                                    };
                                    curr_txn.push(act);
                                    Ok(())
                                }) {
                                Err(_e) => {
                                    // println!("READ ERR -- {:?}", _e);
                                }
                                _ => {}
                            }
                        }
                    }
                    match sqltxn.commit() {
                        Err(_e) => {
                            // println!("COMMIT ERROR {}", _e);
                            // println!("{:?}", curr_txn);
                            curr_txn.clear();
                        }
                        _ => {}
                    }
                    // sqltxn.rollback().unwrap();
                }
                history.push(curr_txn);
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}

#[derive(Clone, Debug)]
pub struct DBNode {
    addr: String,
    id: usize,
}

pub fn single_bench(nodes: &Vec<DBNode>, vars: &Vec<usize>) {
    loop {
        match mysql::Pool::new(nodes[0].addr.clone()) {
            Ok(_conn) => match _conn.get_conn() {
                Ok(mut conn) => {
                    op::create_vars(vars, &mut conn);
                    break;
                }
                Err(e) => {
                    println!("{}", e);
                }
            },
            Err(e) => {
                println!("{}", e);
            }
        }
    }

    let mut threads = Vec::with_capacity(nodes.len());

    let mut session_histories = Vec::with_capacity(nodes.len());

    for i in 0..nodes.len() {
        session_histories.push(Arc::new(Mutex::new(Vec::new())));
        let history = session_histories[i].clone();
        let node_addr = nodes[i].clone();
        let vars = vars.clone();

        threads.push(thread::spawn(move || {
            let mut history = history.lock().unwrap();
            do_single_node(node_addr, &vars, &mut (*history));
        }));
    }

    for t in threads {
        t.join()
            .expect("thread failed to doing bench at a single node");
    }

    let mut histories = Vec::with_capacity(nodes.len());

    for hist in session_histories {
        histories.push(Arc::try_unwrap(hist).unwrap().into_inner().unwrap());
    }

    for (node_id, sess) in histories.iter().enumerate() {
        println!("node {}", node_id + 1);
        for txn in sess.iter() {
            println!("{:?}", txn)
        }
        println!("");
    }

    for sess in histories.iter() {
        for txn in sess.iter() {
            for act in txn.iter() {
                if act.ev == Event::READ {
                    if act.wr_node == 0 {
                        assert_eq!(act.wr_txn, 0);
                        assert_eq!(act.wr_pos, 0);
                    } else {
                        // println!("{:?}", act);
                        let w_act = &histories[act.wr_node - 1][act.wr_txn][act.wr_pos];
                        assert_eq!(act.var, w_act.var);
                        assert_eq!(w_act.ev, Event::WRITE);
                    }
                }
            }
        }
    }

    // add code for serialization check

    let mut txn_last_writes = HashMap::new();

    for (node_id, sess) in histories.iter().enumerate() {
        for (txn_id, txn) in sess.iter().enumerate() {
            let mut last_writes = HashMap::new();
            for act in txn.iter() {
                if act.ev == Event::WRITE {
                    last_writes.insert(act.var, act.wr_pos);
                }
            }
            txn_last_writes.insert((node_id + 1, txn_id), last_writes);
        }
    }

    // checking for non-committed read, non-repeatable read
    for (node_id, sess) in histories.iter().enumerate() {
        for (txn_id, txn) in sess.iter().enumerate() {
            let mut writes = HashMap::new();
            let mut reads: HashMap<usize, (usize, usize, usize)> = HashMap::new();
            for (act_id, act) in txn.iter().enumerate() {
                match act.ev {
                    Event::WRITE => {
                        writes.insert(act.var, act.wr_pos);
                        reads.remove(&act.var);
                    }
                    Event::READ => {
                        if let Some(pos) = writes.get(&act.var) {
                            assert_eq!(txn_id, act.wr_txn, "update-lost!! action-{} of txn({},{}) read value from ({},{},{}) instead from the txn.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                            assert_eq!(node_id + 1, act.wr_node, "update-lost!! action-{} of txn({},{}) read value from ({},{},{}) instead from the txn.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                            assert_eq!(*pos, act.wr_pos, "update-lost!! action-{} of txn({},{}) read value from ({},{},{}) instead from the txn.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                        } else {
                            if act.wr_node != 0 {
                                assert_eq!(
                                    *txn_last_writes
                                        .get(&(act.wr_node, act.wr_txn))
                                        .unwrap()
                                        .get(&act.var)
                                        .unwrap(),
                                    act.wr_pos,
                                    "non-committed read!! action-{} of txn({},{}) read value from ({},{},{}) instead from the txn.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos
                                );
                            }

                            if let Some((wr_node, wr_txn, wr_pos)) = reads.get(&act.var) {
                                assert_eq!(*wr_node, act.wr_node, "non-repeatable read!! action-{} of txn({},{}) read value from ({},{},{}) instead as the last read.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                                assert_eq!(*wr_txn, act.wr_txn, "non-repeatable read!! action-{} of txn({},{}) read value from ({},{},{}) instead as the last read.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                                assert_eq!(*wr_pos, act.wr_pos, "non-repeatable read!! action-{} of txn({},{}) read value from ({},{},{}) instead as the last read.", act_id, node_id + 1, txn_id, act.wr_node, act.wr_txn, act.wr_pos);
                            }
                        }
                        reads.insert(act.var, (act.wr_node, act.wr_txn, act.wr_pos));
                    }
                }
            }
        }
    }

    let n_sizes = histories.iter().map(|ref v| v.len()).collect();
    let mut txn_infos = HashMap::new();

    for (node_id, sess) in histories.iter().enumerate() {
        for (txn_id, txn) in sess.iter().enumerate() {
            let mut rd_info = HashMap::new();
            let mut wr_info = HashSet::new();
            for act in txn.iter() {
                match act.ev {
                    Event::READ => {
                        if act.wr_node != node_id + 1 || act.wr_txn != txn_id {
                            if let Some((old_node, old_txn)) =
                                rd_info.insert(act.var, (act.wr_node, act.wr_txn))
                            {
                                assert_eq!(old_node, act.wr_node);
                                assert_eq!(old_txn, act.wr_txn);
                            }
                        }
                    }
                    Event::WRITE => {
                        wr_info.insert(act.var);
                    }
                }
            }
            txn_infos.insert((node_id + 1, txn_id), (rd_info, wr_info));
        }
    }

    let mut chains = Chains::new(&n_sizes, &txn_infos);
    if !chains.preprocess() {
        println!("found cycle while processing wr and po order");
    }
    println!("{:?}", chains);
    println!("{:?}", chains.serializable_order_dfs());
}

pub fn do_bench() {
    let n_vars = 5;
    let n_nodes = 6;
    // let n_txn = 6;
    // let n_evts_per_txn = 4;
    // let n_iter = 100;

    let nodes = {
        let mut nodes = Vec::with_capacity(n_nodes);
        for i in 1usize..(n_nodes + 1) {
            nodes.push(DBNode {
                addr: format!(
                    "mysql://{}@{}",
                    "root",
                    Ipv4Addr::new(172, 18, 0, 10 + (i as u8))
                ),
                id: i,
            });
        }
        nodes
    };

    {
        let mut conn = mysql::Pool::new(nodes[0].addr.clone())
            .unwrap()
            .get_conn()
            .unwrap();

        op::create_table(&mut conn);
    }

    // for node in nodes.iter() {
    //     let mut conn = mysql::Pool::new(node.addr.clone())
    //         .unwrap()
    //         .get_conn()
    //         .unwrap();
    //     conn.query(format!("SET GLOBAL max_connections = 100000000"))
    //         .unwrap();
    // }

    // return;

    let mut threads = Vec::new();

    for i in 0..1 {
        let nodes = nodes.clone();
        threads.push(thread::spawn(move || {
            single_bench(&nodes, &((i * n_vars)..((i + 1) * n_vars)).collect());
        }));
    }

    for t in threads {
        t.join().expect("failed to single bench");
    }
}
