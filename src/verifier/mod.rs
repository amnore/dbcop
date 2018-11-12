use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use consistency::causal::Causal;
use consistency::sat::Sat;
use consistency::ser::Chains;
use consistency::si::SIChains;
use db::history::Transaction;

use slog::{Drain, Logger};

pub struct Verifier {
    log: slog::Logger,
    dir: PathBuf,
}

impl Verifier {
    pub fn new(dir: PathBuf) -> Self {
        fs::create_dir(&dir).unwrap();
        let log_file = File::create(dir.join("result.log")).unwrap();

        Verifier {
            log: Self::get_logger(BufWriter::new(log_file)),
            dir,
        }
    }

    pub fn get_logger<W>(io: W) -> Logger
    where
        W: Write + Send + 'static,
    {
        let plain = slog_term::PlainSyncDecorator::new(io);
        let root_logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

        info!(root_logger, "Application started";
        "started_at" => format!("{}", chrono::Local::now()));

        root_logger
    }

    pub fn gen_write_map(
        &self,
        histories: &Vec<Vec<Transaction>>,
    ) -> HashMap<(usize, usize), (usize, usize, usize)> {
        let mut write_map = HashMap::new();

        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                for (i_event, event) in transaction.events.iter().enumerate() {
                    if event.write {
                        if let Some(_) = write_map.insert(
                            (event.variable, event.value),
                            (i_node + 1, i_transaction, i_event),
                        ) {
                            unreachable!();
                        }
                    }
                    write_map.entry((event.variable, 0)).or_insert((0, 0, 0));
                }
            }
        }

        write_map
    }

    pub fn transactional_history_verify(&self, histories: &Vec<Vec<Transaction>>) {
        let write_map = self.gen_write_map(histories);

        for (i_node_r, session) in histories.iter().enumerate() {
            for (i_transaction_r, transaction) in session.iter().enumerate() {
                if transaction.success {
                    for (i_event_r, event) in transaction.events.iter().enumerate() {
                        if !event.write && event.success {
                            if let Some(&(i_node, i_transaction, i_event)) =
                                write_map.get(&(event.variable, event.value))
                            {
                                if event.value == 0 {
                                    assert_eq!(i_node, 0);
                                    assert_eq!(i_transaction, 0);
                                    assert_eq!(i_event, 0);
                                } else {
                                    let transaction2 = &histories[i_node - 1][i_transaction];
                                    // let event2 = &transaction2.events[i_event];
                                    // info!(self.log,"{:?}\n{:?}", event, event2);
                                    if !transaction2.success {
                                        info!(
                                            self.log,
                                            "{:?} read from {:?}",
                                            (i_node_r + 1, i_transaction_r, i_event_r),
                                            (i_node, i_transaction, i_event),
                                        );
                                        info!(self.log, "DIRTY READ");
                                        return;
                                    }
                                }
                            } else {
                                info!(self.log, "NO WRITE WITH SAME (VARIABLE, VALUE)");
                                return;
                            }
                        }
                    }
                }
            }
        }

        // add code for serialization check

        let mut transaction_last_writes = HashMap::new();

        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                if transaction.success {
                    let mut last_writes = HashMap::new();
                    for (i_event, event) in transaction.events.iter().enumerate() {
                        if event.write && event.success {
                            // goes first to last, so when finished, it is last write event
                            last_writes.insert(event.variable, i_event);
                        }
                    }
                    transaction_last_writes.insert((i_node + 1, i_transaction), last_writes);
                }
            }
        }

        // checking for non-committed read, non-repeatable read
        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                let mut writes = HashMap::new();
                let mut reads: HashMap<usize, (usize, usize, usize)> = HashMap::new();
                if transaction.success {
                    for (i_event, event) in transaction.events.iter().enumerate() {
                        if event.success {
                            if event.write {
                                writes.insert(event.variable, i_event);
                                reads.remove(&event.variable);
                            } else {
                                let &(wr_i_node, wr_i_transaction, wr_i_event) =
                                    write_map.get(&(event.variable, event.value)).unwrap();
                                if let Some(pos) = writes.get(&event.variable) {
                                    // checking if read the last write in same transaction
                                    if !((i_node + 1 == wr_i_node)
                                        && (i_transaction == wr_i_transaction)
                                        && (*pos == wr_i_event))
                                    {
                                        info!(
                                            self.log,
                                            "wr:{:?}, rd:{:?}",
                                            (wr_i_node, wr_i_transaction, wr_i_event),
                                            (i_node + 1, i_transaction, i_event)
                                        );
                                        info!(self.log, "LOST UPDATE");
                                        return;
                                    }
                                // assert!(
                                //     (i_node + 1 == wr_i_node) && (i_transaction == wr_i_transaction) && (*pos == wr_i_event),
                                //     "update-lost!! event-{} of txn({},{}) read value from ({},{},{}) instead from the txn.",
                                //     i_event,
                                //     i_node + 1,
                                //     i_transaction,
                                //     wr_i_node,
                                //     wr_i_transaction,
                                //     wr_i_event
                                // );
                                } else {
                                    if event.value != 0 {
                                        // checking if read the last write from other transaction
                                        if *transaction_last_writes
                                            .get(&(wr_i_node, wr_i_transaction))
                                            .unwrap()
                                            .get(&event.variable)
                                            .unwrap()
                                            != wr_i_event
                                        {
                                            info!(self.log, "UNCOMMITTED READ");
                                            return;
                                        }
                                        // assert_eq!(
                                        //     *transaction_last_writes
                                        //         .get(&(wr_i_node, wr_i_transaction))
                                        //         .unwrap()
                                        //         .get(&event.variable)
                                        //         .unwrap(),
                                        //     wr_i_event,
                                        //     "non-committed read!! action-{} of txn({},{}) read value from ({},{},{}) instead from the txn.",
                                        //     i_event,
                                        //     i_node + 1,
                                        //     i_transaction,
                                        //     wr_i_node,
                                        //     wr_i_transaction,
                                        //     wr_i_event
                                        // );
                                    }

                                    if let Some((wr_i_node2, wr_i_transaction2, wr_i_event2)) =
                                        reads.get(&event.variable)
                                    {
                                        // checking if the read the same write as the last read in same transaction
                                        if !((*wr_i_node2 == wr_i_node)
                                            && (*wr_i_transaction2 == wr_i_transaction)
                                            && (*wr_i_event2 == wr_i_event))
                                        {
                                            info!(self.log, "NON REPEATABLE READ");
                                            return;
                                        }
                                        // assert!(
                                        //     (*wr_i_node2 == wr_i_node) && (*wr_i_transaction2 == wr_i_transaction) && (*wr_i_event2 == wr_i_event),
                                        //     "non-repeatable read!! action-{} of txn({},{}) read value from ({},{},{}) instead as the last read.",
                                        //     i_event,
                                        //     i_node + 1,
                                        //     i_transaction,
                                        //     wr_i_node,
                                        //     wr_i_transaction,
                                        //     wr_i_event,
                                        // )
                                    }
                                }
                                reads.insert(
                                    event.variable,
                                    (wr_i_node, wr_i_transaction, wr_i_event),
                                );
                            }
                        }
                    }
                }
            }
        }

        let n_sizes: Vec<_> = histories.iter().map(|ref v| v.len()).collect();
        let mut transaction_infos = HashMap::new();

        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                let mut read_info = HashMap::new();
                let mut write_info = HashSet::new();
                if transaction.success {
                    for event in transaction.events.iter() {
                        if event.success {
                            if event.write {
                                write_info.insert(event.variable);
                            } else {
                                let &(wr_i_node, wr_i_transaction, _) =
                                    write_map.get(&(event.variable, event.value)).unwrap();
                                if wr_i_node != i_node + 1 || wr_i_transaction != i_transaction {
                                    if let Some((old_i_node, old_i_transaction)) = read_info
                                        .insert(event.variable, (wr_i_node, wr_i_transaction))
                                    {
                                        // should be same, because repeatable read
                                        assert_eq!(old_i_node, wr_i_node);
                                        assert_eq!(old_i_transaction, wr_i_transaction);
                                    }
                                }
                            }
                        }
                    }
                }
                transaction_infos.insert((i_node + 1, i_transaction), (read_info, write_info));
            }
        }

        if true {
            let sat_time = std::time::Instant::now();
            let mut sat_solver = Sat::new(&n_sizes, &transaction_infos);

            sat_solver.pre_vis_co();
            sat_solver.session();
            sat_solver.wr_ww_rw();
            sat_solver.vis_transitive();

            info!(self.log, "SAT DECISION START");

            // CC
            sat_solver.causal();
            if sat_solver.solve(&self.dir) {
                // prefix
                sat_solver.prefix();
                if sat_solver.solve(&self.dir) {
                    // SI
                    sat_solver.conflict();
                    if sat_solver.solve(&self.dir) {
                        // SER
                        sat_solver.ser();
                        if sat_solver.solve(&self.dir) {
                            info!(self.log, "SER")
                        } else {
                            info!(self.log, "SI, NON SER");
                        }
                    } else {
                        info!(self.log, "PRE, but NON-SI");
                    }
                } else {
                    info!(self.log, "CC, but NON-PRE");
                }
            } else {
                info!(self.log, "NON-CC")
            }

            info!(self.log, "SAT DECISION END");

            let sat_dur = sat_time.elapsed();

            info!(
                self.log,
                "SAT DECISION TOOK {:?}secs",
                sat_dur.as_secs() as f64 + sat_dur.subsec_nanos() as f64 * 1e-9
            );
        }

        {
            let algo_time = std::time::Instant::now();

            {
                info!(self.log, "Doing causal consistency check");
                let mut causal = Causal::new(&n_sizes, &transaction_infos, self.log.clone());
                if causal.preprocess_vis() && causal.preprocess_co() {
                    info!(self.log, "History is causal consistent!");
                    info!(self.log, "CC");
                    if true {
                        // info!(self.log);
                        info!(self.log, "Doing serializable consistency check");
                        let mut chains =
                            Chains::new(&n_sizes, &transaction_infos, self.log.clone());
                        info!(self.log, "{:?}", chains);
                        if !chains.preprocess() {
                            info!(self.log, "found cycle while processing wr and po order");
                        }
                        // info!(self.log,"{:?}", chains);
                        // info!(self.log,"{:?}", chains.serializable_order_dfs());
                        match chains.serializable_order_dfs() {
                            Some(order) => {
                                info!(self.log, "Serializable progress of transactions");
                                let mut order_vec = Vec::new();
                                for node_id in order {
                                    order_vec.push(node_id);
                                }
                                info!(self.log, "{:?}", order_vec);
                                info!(self.log, "SER")
                            }
                            None => {
                                info!(self.log, "No valid SER history");
                                //info!(self.log,);
                                {
                                    info!(self.log, "Doing snapshot isolation check");
                                    let mut chains = SIChains::new(
                                        &n_sizes,
                                        &transaction_infos,
                                        self.log.clone(),
                                    );
                                    info!(self.log, "{:?}", chains);
                                    if !chains.preprocess() {
                                        info!(
                                            self.log,
                                            "found cycle while processing wr and po order"
                                        );
                                    }
                                    // info!(self.log,"{:?}", chains);
                                    match chains.serializable_order_dfs() {
                                        Some(order) => {
                                            let mut rw_map = HashMap::new();
                                            info!(self.log,
                                        "SI progress of transactions (broken in read and write)"
                                    );
                                            let mut order_vec = Vec::new();
                                            for node_id in order {
                                                let ent = rw_map.entry(node_id).or_insert(true);
                                                if *ent {
                                                    order_vec.push(format!("{}R", node_id));
                                                    *ent = false;
                                                } else {
                                                    order_vec.push(format!("{}W", node_id));
                                                    *ent = true;
                                                }
                                            }
                                            info!(self.log, "{:?}", order_vec);
                                            info!(self.log, "SI")
                                        }
                                        None => info!(self.log, "No valid SI history\nNON-SI"),
                                    }
                                }
                            }
                        }
                    }
                } else {
                    info!(self.log, "no valid causal consistent history");
                    info!(self.log, "NON-CC");
                }
            }

            let algo_dur = algo_time.elapsed();

            info!(
                self.log,
                "ALGO DECISION TOOK {:?}secs",
                algo_dur.as_secs() as f64 + algo_dur.subsec_nanos() as f64 * 1e-9
            );
        }
    }
}
