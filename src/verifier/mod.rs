mod consistency;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
// use std::fs;
use std::fs::File;
use std::path::PathBuf;

// use consistency::sat::Sat;
use consistency::Consistency;
use crate::db::history::{Session, Event};

use consistency::algo::{
    AtomicHistoryPO, PrefixConsistentHistory, SerializableHistory, SnapshotIsolationHistory,
};
use consistency::util::ConstrainedLinearization;

mod util;

use self::util::{BiConn, UGraph};

pub struct Verifier {
    consistency_model: Consistency,
    use_sat: bool,
    use_bicomponent: bool,
    dir: PathBuf,
}

impl Verifier {
    pub fn new(dir: &PathBuf) -> Self {
        // fs::create_dir(&dir).unwrap();
        let log_file = File::create(dir.join("result_log.json")).unwrap();

        Verifier {
            consistency_model: Consistency::Serializable,
            use_sat: false,
            use_bicomponent: false,
            dir: dir.clone(),
        }
    }

    pub fn model(&mut self, model: &str) {
        self.consistency_model = match model {
            "rc" => Consistency::ReadCommitted,
            "rr" => Consistency::RepeatableRead,
            "ra" => Consistency::ReadAtomic,
            "cc" => Consistency::Causal,
            "pre" => Consistency::Prefix,
            "si" => Consistency::SnapshotIsolation,
            "ser" => Consistency::Serializable,
            "lin" => Consistency::Linearizable,
            "" => Consistency::Inc,
            &_ => unreachable!(),
        }
    }

    pub fn sat(&mut self, flag: bool) {
        self.use_sat = flag;
    }

    pub fn bicomponent(&mut self, flag: bool) {
        self.use_bicomponent = flag;
    }

    pub fn gen_write_map(histories: &[Session]) -> HashMap<(usize, usize), (usize, usize, usize)> {
        let mut write_map = HashMap::new();

        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                for (i_event, event) in transaction.events.iter().enumerate() {
                    if event.write {
                        if write_map
                            .insert(
                                (event.variable, event.value),
                                (i_node + 1, i_transaction, i_event),
                            )
                            .is_some()
                        {
                            panic!("each write should be unique");
                        }
                    } else {
                        write_map.entry((event.variable, 0)).or_insert((0, 0, 0));
                    }
                }
            }
        }

        write_map
    }

    pub fn verify(&mut self, histories: &[Session]) -> Option<Consistency> {
        let moment = std::time::Instant::now();
        let decision = self.transactional_history_verify(histories);
        let duration = moment.elapsed();

        eprintln!(
            "INFO: the algorithm finished, model={:?}, sat={:?}, bicomponent={:?}, duration={:?}, minViolation={:?}",
            self.consistency_model, self.use_sat, self.use_bicomponent, duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) * 1e-9,match decision {
                    Some(e) => format!("{:?}",e),
                    None => format!("ok")
                }
        );

        decision
    }

    fn verify_history_linearizable(&self, histories: &[Session]) -> bool {
        let mut operations = histories.iter().flatten().flat_map(|t| {
            let mut rmw_ops = Vec::new();
            let mut read_op: RefCell<Option<&Event>> = RefCell::new(None);

            for ev in &t.events {
                if ev.write {
                    let read_ev = read_op.take().unwrap();
                    assert_eq!(read_ev.variable, ev.variable);
                    assert!(read_ev.success && ev.success);

                    rmw_ops.push(Event {
                        write: true,
                        variable: ev.variable,
                        value: ev.value,
                        success: true,
                        start_time: read_ev.start_time,
                        end_time: ev.end_time,
                    });
                } else {
                    read_op.replace(Some(ev));
                }
            }

            rmw_ops.into_iter()
        }).collect::<Vec<_>>();
        operations.sort_by_key(|e| e.start_time);
        self.time_valid(&operations, 0, operations.len())
    }

    fn time_valid(&self, events: &Vec<Event>, i: usize, j: usize) -> bool {
        if i >= j {
            return true;
        }

        let first = &events[i];
        for k in i+1..j {
            if first.start_time >= events[k].end_time {
                return false;
            }
        }

        self.time_valid(events, i+1, j)
    }

    pub fn transactional_history_verify(&mut self, histories: &[Session]) -> Option<Consistency> {
        let write_map = Self::gen_write_map(histories);

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
                                        eprintln!("{:?} read from {:?}", (i_node_r + 1, i_transaction_r, i_event_r),
                                            (i_node, i_transaction, i_event),
                                        );
                                        eprintln!("finished early, reason=DIRTY READ, description=read from uncommitted/aborted transaction");
                                        return Some(Consistency::ReadCommitted);
                                    }
                                }
                            } else {
                                eprintln!("finished early, reason=NO WRITE WITH SAME (VARIABLE, VALUE)");
                                panic!("In consistent write");
                                // return false;
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
                                        eprintln!(
                                            "wr:{:?}, rd:{:?}",
                                            (wr_i_node, wr_i_transaction, wr_i_event),
                                            (i_node + 1, i_transaction, i_event)
                                        );
                                        eprintln!("finished early, reason=LOST UPDATE, description=did not read the latest write within transaction");
                                        return Some(Consistency::ReadCommitted);
                                    }
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
                                            eprintln!("finished early, reason=UNCOMMITTED READ, description=read some non-last write from other transaction");
                                            return Some(Consistency::ReadCommitted);
                                        }
                                    }

                                    if let Some((wr_i_node2, wr_i_transaction2, wr_i_event2)) =
                                        reads.get(&event.variable)
                                    {
                                        // checking if the read the same write as the last read in same transaction
                                        if !((*wr_i_node2 == wr_i_node)
                                            && (*wr_i_transaction2 == wr_i_transaction)
                                            && (*wr_i_event2 == wr_i_event))
                                        {
                                            eprintln!("finished early, reason=NON REPEATABLE READ, description=did not read same as latest read which is after lastest write");
                                            return Some(Consistency::RepeatableRead);
                                        }
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

        eprintln!("each read from latest write");
        eprintln!("atomic reads");

        let mut transaction_infos = HashMap::new();

        let mut root_write_info = HashSet::new();

        for (i_node, session) in histories.iter().enumerate() {
            for (i_transaction, transaction) in session.iter().enumerate() {
                let mut read_info = HashMap::new();
                let mut write_info = HashSet::new();
                if transaction.success {
                    for event in transaction.events.iter() {
                        if event.success {
                            if event.write {
                                write_info.insert(event.variable);
                                // all variable is initialized at root transaction
                                root_write_info.insert(event.variable);
                            } else {
                                let &(wr_i_node, wr_i_transaction, _) =
                                    write_map.get(&(event.variable, event.value)).unwrap();
                                if event.value == 0 {
                                    assert_eq!(wr_i_node, 0);
                                    assert_eq!(wr_i_transaction, 0);
                                    root_write_info.insert(event.variable);
                                }
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
                if !read_info.is_empty() || !write_info.is_empty() {
                    transaction_infos.insert((i_node + 1, i_transaction), (read_info, write_info));
                }
            }
        }

        if !root_write_info.is_empty() {
            assert!(transaction_infos
                .insert((0, 0), (Default::default(), root_write_info))
                .is_none());
        }

        eprintln!("atleast not read commmitted, number of transactions={}", transaction_infos.len());

        if self.use_sat {
            eprintln!("using SAT");
        }

        if self.use_bicomponent {
            eprintln!("using bicomponent");
        }

        if self.use_bicomponent {
            // communication graph
            eprintln!("doing bicomponent decomposition");
            let mut access_map = HashMap::new();
            {
                let mut access_vars = HashSet::new();
                for (i_node, session) in histories.iter().enumerate() {
                    for transaction in session.iter() {
                        if transaction.success {
                            for event in transaction.events.iter() {
                                if event.success {
                                    access_vars.insert(event.variable);
                                }
                            }
                        }
                    }
                    for x in access_vars.drain() {
                        access_map
                            .entry(x)
                            .or_insert_with(Vec::new)
                            .push(i_node + 1);
                    }
                }
            }

            let mut ug: UGraph<usize> = Default::default();

            for (_, ss) in access_map.drain() {
                for &s1 in ss.iter() {
                    for &s2 in ss.iter() {
                        if s1 != s2 {
                            ug.add_edge(s1, s2);
                        }
                    }
                }
            }

            let biconn = BiConn::new(ug);

            let biconnected_components = biconn.get_biconnected_vertex_components();

            if biconnected_components.iter().all(|component| {
                eprintln!("doing for component {:?}", component);
                let restrict_infos = self.restrict(&transaction_infos, component);

                self.do_hard_verification(&histories, &restrict_infos).is_none()
            }) {
                None
            } else {
                Some(self.consistency_model)
            }
        } else {
            self.do_hard_verification(histories, &transaction_infos)
        }
    }

    fn restrict(
        &self,
        transaction_infos: &HashMap<
            (usize, usize),
            (HashMap<usize, (usize, usize)>, HashSet<usize>),
        >,
        component: &HashSet<usize>,
    ) -> HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)> {
        let mut new_info = transaction_infos.clone();

        new_info.retain(|k, _| component.contains(&k.0));

        new_info
            .values_mut()
            .for_each(|(read_info, _)| read_info.retain(|_, k| component.contains(&k.0)));

        new_info
    }

    fn do_hard_verification(
        &mut self,
        histories: &[Session],
        transaction_infos: &HashMap<
            (usize, usize),
            (HashMap<usize, (usize, usize)>, HashSet<usize>),
        >,
    ) -> Option<Consistency> {
        if self.use_sat {
            unimplemented!("minisat v0.4.4 conflicts with mysql v22.2.0");
            // let mut sat_solver = Sat::new(&transaction_infos);

            // sat_solver.pre_vis_co();
            // sat_solver.session();
            // sat_solver.wr();
            // sat_solver.read_atomic();

            // match self.consistency_model {
            //     Consistency::Causal => {
            //         sat_solver.vis_transitive();
            //     }
            //     Consistency::Prefix => {
            //         sat_solver.prefix();
            //     }
            //     Consistency::SnapshotIsolation => {
            //         sat_solver.prefix();
            //         sat_solver.conflict();
            //     }
            //     Consistency::Serializable => {
            //         sat_solver.ser();
            //     }
            //     _ => unreachable!(),
            // }

            // if sat_solver.solve().is_some() {
            //     None
            // } else {
            //     Some(self.consistency_model)
            // }
        } else {
            eprintln!("using our algorithms");

            match self.consistency_model {
                Consistency::ReadAtomic => {
                    let mut ra_hist = AtomicHistoryPO::new(transaction_infos.clone());

                    let wr = ra_hist.get_wr();
                    ra_hist.vis_includes(&wr);
                    // ra_hist.vis_is_trans();
                    let ww = ra_hist.causal_ww();
                    for (_, ww_x) in ww.iter() {
                        ra_hist.vis_includes(ww_x);
                    }
                    // ra_hist.vis_is_trans();

                    if ra_hist.vis.has_cycle() {
                        Some(self.consistency_model)
                    } else {
                        None
                    }
                }
                Consistency::Causal => {
                    let mut causal_hist = AtomicHistoryPO::new(transaction_infos.clone());

                    let wr = causal_hist.get_wr();
                    causal_hist.vis_includes(&wr);
                    causal_hist.vis_is_trans();
                    let ww = causal_hist.causal_ww();
                    for (_, ww_x) in ww.iter() {
                        causal_hist.vis_includes(ww_x);
                    }
                    causal_hist.vis_is_trans();

                    if causal_hist.vis.has_cycle() {
                        Some(self.consistency_model)
                    } else {
                        None
                    }
                }
                Consistency::Prefix => {
                    let mut pre_hist =
                        PrefixConsistentHistory::new(transaction_infos.clone());

                    let wr = pre_hist.history.get_wr();
                    pre_hist.history.vis_includes(&wr);
                    pre_hist.history.vis_is_trans();
                    let ww = pre_hist.history.causal_ww();
                    for (_, ww_x) in ww.iter() {
                        pre_hist.history.vis_includes(ww_x);
                    }
                    pre_hist.history.vis_is_trans();

                    if pre_hist.history.vis.has_cycle() {
                        Some(self.consistency_model)
                    } else {
                        if pre_hist.get_linearization().is_some() {
                            None
                        } else {
                            Some(self.consistency_model)
                        }
                    }
                }
                Consistency::SnapshotIsolation => {
                    let mut si_hist =
                        SnapshotIsolationHistory::new(transaction_infos.clone());

                    let wr = si_hist.history.get_wr();
                    si_hist.history.vis_includes(&wr);
                    si_hist.history.vis_is_trans();
                    let ww = si_hist.history.causal_ww();
                    for (_, ww_x) in ww.iter() {
                        si_hist.history.vis_includes(ww_x);
                    }
                    si_hist.history.vis_is_trans();

                    if si_hist.history.vis.has_cycle() {
                        Some(self.consistency_model)
                    } else {
                        if si_hist.get_linearization().is_some() {
                            None
                        } else {
                            Some(self.consistency_model)
                        }
                    }
                }
                Consistency::Serializable => {
                    let mut ser_hist =
                        SerializableHistory::new(transaction_infos.clone());

                    let wr = ser_hist.history.get_wr();
                    ser_hist.history.vis_includes(&wr);
                    let mut change = false;
                    // wsc code
                    let mut now = std::time::Instant::now();
                    // println!("wsc start");
                    loop {
                        change |= ser_hist.history.vis_is_trans();
                        if !change {
                            break;
                        } else {
                            change = false;
                        }
                        let ww = ser_hist.history.causal_ww();
                        for (_, ww_x) in ww.iter() {
                            change |= ser_hist.history.vis_includes(ww_x);
                        }
                        let rw = ser_hist.history.causal_rw();
                        for (_, rw_x) in rw.iter() {
                            change |= ser_hist.history.vis_includes(rw_x);
                        }
                    }
                    // println!("wsc end");
                    // println!("wsc took {}secs", now.elapsed().as_secs());

                    if ser_hist.history.vis.has_cycle() {
                        Some(self.consistency_model)
                    } else {
                        // let lin_o = ser_hist.get_linearization();
                        // {
                        //     // checking correctness
                        //     if let Some(ref lin) = lin_o {
                        //         let mut curr_value_map: HashMap<usize, (usize, usize)> =
                        //             Default::default();
                        //         for txn_id in lin.iter() {
                        //             let (read_info, write_info) =
                        //                 transaction_infos.get(txn_id).unwrap();
                        //             for (x, txn1) in read_info.iter() {
                        //                 match curr_value_map.get(&x) {
                        //                     Some(txn1_) => assert_eq!(txn1_, txn1),
                        //                     _ => unreachable!(),
                        //                 }
                        //             }
                        //             for &x in write_info.iter() {
                        //                 curr_value_map.insert(x, *txn_id);
                        //             }
                        //             // if !write_info.is_empty() {
                        //             //     println!("{:?}", txn_id);
                        //             //     println!("{:?}", curr_value_map);
                        //             // }
                        //         }
                        //     }
                        // }
                        // lin_o.is_some();

                        now = std::time::Instant::now();
                        if ser_hist.get_linearization().is_some() {
                            // println!("dbcop main algorithm took {}secs", now.elapsed().as_secs());
                            None
                        } else {
                            Some(self.consistency_model)
                        }
                    }
                }
                Consistency::Linearizable => {
                    if self.verify_history_linearizable(histories) {
                        None
                    } else {
                        Some(Consistency::Linearizable)
                    }
                }
                Consistency::Inc => {
                    self.consistency_model = Consistency::ReadAtomic;
                    let decision = self.do_hard_verification(&histories, transaction_infos);
                    if decision.is_some() {
                        return decision;
                    }
                    self.consistency_model = Consistency::Causal;
                    let decision = self.do_hard_verification(&histories, transaction_infos);
                    if decision.is_some() {
                        return decision;
                    }
                    self.consistency_model = Consistency::Prefix;
                    let decision = self.do_hard_verification(&histories, transaction_infos);
                    if decision.is_some() {
                        return decision;
                    }
                    self.consistency_model = Consistency::SnapshotIsolation;
                    let decision = self.do_hard_verification(&histories, transaction_infos);
                    if decision.is_some() {
                        return decision;
                    }
                    self.consistency_model = Consistency::Serializable;
                    let decision = self.do_hard_verification(&histories, transaction_infos);
                    if decision.is_some() {
                        return decision;
                    }
                    self.consistency_model = Consistency::Inc;
                    None
                }
                _ => {
                    unreachable!();
                }
            }
        }
    }
}
