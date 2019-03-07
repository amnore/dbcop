use hashbrown::{HashMap, HashSet};
use std::fs::{File, OpenOptions};

use std::process::{Command, Stdio};

use std::path::PathBuf;

use std::io::BufRead;
use std::io::BufReader;

use std::io::Write;

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Edge {
    CO,
    VI,
    WW(usize),
}

#[derive(Default, Debug)]
struct CNF {
    cnf_string: Vec<u8>,
    n_clause: usize,
    n_variable: usize,
}

impl CNF {
    fn add_variable(&mut self, var: usize, sign: bool) {
        self.n_variable = std::cmp::max(self.n_variable, var);
        if sign {
            write!(self.cnf_string, "{} ", var).expect("cnf write failed");
        } else {
            write!(self.cnf_string, "-{} ", var).expect("cnf write failed");
        }
    }

    fn finish_clause(&mut self) {
        writeln!(self.cnf_string, " 0").expect("cnf write failed");
        self.n_clause += 1;
    }

    fn write_to_file(&self, path: &PathBuf) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .expect("couldn't create");

        writeln!(file, "p cnf {} {}", self.n_variable, self.n_clause)
            .expect("failed to write parameters");
        file.write_all(&self.cnf_string)
            .expect("failed to write clauses");
    }
}

#[derive(Debug)]
pub struct Sat {
    cnf: CNF,
    edge_variable: HashMap<(Edge, (usize, usize), (usize, usize)), usize>,
    write_variable: HashMap<usize, HashMap<(usize, usize), HashSet<(usize, usize)>>>,
    n_sizes: Vec<usize>,
    transactions: Vec<(usize, usize)>,
}

impl Sat {
    pub fn new(
        n_sizes: &[usize],
        txns_info: &HashMap<(usize, usize), (HashMap<usize, (usize, usize)>, HashSet<usize>)>,
    ) -> Self {
        let mut write_variable: HashMap<usize, HashMap<(usize, usize), HashSet<(usize, usize)>>> =
            HashMap::new();

        for (&transaction1, (ref read_info, write_info)) in txns_info.iter() {
            for &x in write_info.iter() {
                let entry = write_variable.entry(x).or_insert_with(Default::default);
                entry.entry(transaction1).or_insert_with(Default::default);
            }
            for (&x, &transaction2) in read_info.iter() {
                let entry1 = write_variable.entry(x).or_insert_with(Default::default);
                let entry2 = entry1.entry(transaction2).or_insert_with(Default::default);
                entry2.insert(transaction1);
            }
        }

        for (_, mut wr_map) in write_variable.iter_mut() {
            wr_map.entry((0, 0)).or_insert_with(Default::default);
        }

        let mut transactions = vec![(0, 0)];

        for (i_node, &n_transaction) in n_sizes.iter().enumerate() {
            for i_transaction in 0..n_transaction {
                transactions.push((i_node + 1, i_transaction));
            }
        }

        Sat {
            cnf: Default::default(),
            edge_variable: HashMap::new(),
            write_variable,
            n_sizes: n_sizes.to_owned(),
            transactions,
        }
    }

    pub fn session(&mut self) {
        let mut clauses = Vec::new();
        for (i_node, &n_transaction) in self.n_sizes.iter().enumerate() {
            for i_transaction in 1..n_transaction {
                // session orders
                clauses.push(vec![(
                    Edge::VI,
                    (i_node + 1, i_transaction - 1),
                    (i_node + 1, i_transaction),
                    true,
                )])
            }
            clauses.push(vec![(Edge::VI, (0, 0), (i_node + 1, 0), true)]);
        }

        self.add_clauses(&clauses);
    }

    pub fn pre_vis_co(&mut self) {
        let mut clauses = Vec::new();

        for &t1 in self.transactions.iter() {
            for &t2 in self.transactions.iter() {
                if t1 != t2 {
                    // VIS <= CO
                    clauses.push(vec![(Edge::VI, t1, t2, false), (Edge::CO, t1, t2, true)]);

                    // CO total
                    // no cycle
                    clauses.push(vec![(Edge::CO, t1, t2, false), (Edge::CO, t2, t1, false)]);
                    // total
                    clauses.push(vec![(Edge::CO, t1, t2, true), (Edge::CO, t2, t1, true)]);

                    for &t3 in self.transactions.iter() {
                        if t2 != t3 {
                            // CO transitive / CO;CO => CO
                            clauses.push(vec![
                                (Edge::CO, t1, t2, false),
                                (Edge::CO, t2, t3, false),
                                (Edge::CO, t1, t3, true),
                            ]);
                        }
                    }
                }
            }
        }
        self.add_clauses(&clauses);
    }

    pub fn ser(&mut self) {
        let mut clauses = Vec::new();

        for &t1 in self.transactions.iter() {
            for &t2 in self.transactions.iter() {
                if t1 != t2 {
                    // CO <= VIS
                    clauses.push(vec![(Edge::CO, t1, t2, false), (Edge::VI, t1, t2, true)]);
                }
            }
        }
        self.add_clauses(&clauses);
    }

    pub fn vis_transitive(&mut self) {
        let mut clauses = Vec::new();

        for &t1 in self.transactions.iter() {
            for &t2 in self.transactions.iter() {
                if t1 != t2 {
                    for &t3 in self.transactions.iter() {
                        if t2 != t3 {
                            // VI transitive / VI;VI => VI
                            clauses.push(vec![
                                (Edge::VI, t1, t2, false),
                                (Edge::VI, t2, t3, false),
                                (Edge::VI, t1, t3, true),
                            ]);
                        }
                    }
                }
            }
        }
        self.add_clauses(&clauses);
    }

    pub fn wr_ww(&mut self) {
        let mut clauses = Vec::new();

        for (&x, ref wr_map) in self.write_variable.iter() {
            for (&u1, ref vs) in wr_map.iter() {
                for &v in vs.iter() {
                    // clauses.push(vec![(Edge::WR(x), u1, v, true)]);
                    clauses.push(vec![(Edge::VI, u1, v, true)]);
                }
                for (&u2, _) in wr_map.iter() {
                    if u1 != u2 {
                        clauses.push(vec![(Edge::WW(x), u1, u2, false), (Edge::CO, u1, u2, true)]);
                        clauses.push(vec![
                            (Edge::WW(x), u1, u2, true),
                            (Edge::WW(x), u2, u1, true),
                        ]);
                    }
                }
            }
        }

        self.add_clauses(&clauses);
    }

    pub fn read_atomic(&mut self) {
        let mut clauses = Vec::new();

        for (&x, ref wr_map) in self.write_variable.iter() {
            for (&u, ref vs) in wr_map.iter() {
                for &v in vs.iter() {
                    for (&u1, _) in wr_map.iter() {
                        if u1 != u {
                            clauses
                                .push(vec![(Edge::VI, u1, v, false), (Edge::WW(x), u1, u, true)]);
                        }
                    }
                }
            }
        }

        self.add_clauses(&clauses);
    }

    pub fn prefix(&mut self) {
        let mut clauses = Vec::new();

        for &t1 in self.transactions.iter() {
            for &t2 in self.transactions.iter() {
                if t1 != t2 {
                    for &t3 in self.transactions.iter() {
                        if t2 != t3 {
                            // CO;VI => VI
                            clauses.push(vec![
                                (Edge::CO, t1, t2, false),
                                (Edge::VI, t2, t3, false),
                                (Edge::VI, t1, t3, true),
                            ]);
                        }
                    }
                }
            }
        }
        self.add_clauses(&clauses);
    }

    pub fn conflict(&mut self) {
        let mut clauses = Vec::new();
        for (&x, ref wr_map) in self.write_variable.iter() {
            for (&u1, _) in wr_map.iter() {
                for (&u2, _) in wr_map.iter() {
                    if u1 != u2 {
                        clauses.push(vec![(Edge::WW(x), u1, u2, false), (Edge::VI, u1, u2, true)]);
                    }
                }
            }
        }
        self.add_clauses(&clauses);
    }

    pub fn solve(&self, path: &PathBuf) -> bool {
        let inp_cnf = path.join("history.cnf");
        let out_cnf = path.join("result.cnf");
        self.cnf.write_to_file(&inp_cnf);

        if let Ok(mut child) = Command::new("minisat")
            .arg(&inp_cnf)
            .arg(&out_cnf)
            .stdout(Stdio::null())
            .spawn()
        {
            child.wait().expect("failed to execute process");
        } else {
            panic!("failed to execute process")
        }

        // println!("status: {}", output.status);
        // println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        // println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        let result = File::open(&out_cnf).expect("file couldn't open");

        let reader = BufReader::new(&result);

        let mut lines = reader.lines().map(|l| l.unwrap());

        let mut assignments = HashMap::new();

        match lines.next() {
            Some(ref e) if e.as_str() == "SAT" => {
                for line in lines {
                    for var_st in line.split_whitespace() {
                        let var: isize = var_st.parse().unwrap();
                        if var != 0 {
                            assignments.insert(var.abs() as usize, var > 0);
                        }
                    }
                }
            }
            Some(ref e) if e.as_str() == "UNSAT" => {
                // println!("{:?}", e);
                // for line in lines {
                //     println!("{}", line);
                // }
            }
            _ => {
                unreachable!();
            }
        }

        if !assignments.is_empty() {
            let mut edges: Vec<_> = self
                .edge_variable
                .iter()
                .filter(|(_, &v)| assignments[&v])
                .map(|(&k, _)| k)
                .collect();

            edges.sort_unstable();

            for e in &edges {
                if e.0 == Edge::CO {
                    println!("{:?}", e);
                }
            }

            true
        } else {
            false
        }
    }

    pub fn add_clause(&mut self, edges: &[(Edge, (usize, usize), (usize, usize), bool)]) {
        for edge in edges.iter() {
            let variable = self.get_variable(edge.0, edge.1, edge.2);
            self.cnf.add_variable(variable, edge.3);
        }
        self.cnf.finish_clause();
    }

    pub fn add_clauses(&mut self, clauses: &[Vec<(Edge, (usize, usize), (usize, usize), bool)>]) {
        for clause in clauses.iter() {
            self.add_clause(clause);
        }
    }

    pub fn get_variable(&mut self, edge: Edge, u: (usize, usize), v: (usize, usize)) -> usize {
        let usable = self.edge_variable.len() + 1;
        *self.edge_variable.entry((edge, u, v)).or_insert(usable)
    }
}
