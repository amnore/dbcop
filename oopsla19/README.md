# OOPSLA '19 Artifact Submission

1.  We have generated histories for 3 databases.

    -   [CockroachDB](https://www.cockroachlabs.com/)
    -   [Galera cluster](https://galeracluster.com)
    -   [AntidoteDB](https://www.antidotedb.eu/)

2.  We used 6 sessions, 30 transactions per sessions, 20 operations per transactions and 6 average variables per session as default and varied one fixed parameter.

    -   CockroachDB for serialization.
        Sessions {3, 6, 9, 12, 15}
        Transactions {20, 30, 40, 50, 60}
        Operations {10, 20, 30, 40, 50}
        Average variable per session {50, 60, 70, 80, 90}
            
    -   CockroachDB for Snapshot Isolation
        Sessions {3, 6, 9, 12, 15}
            
    -   Galera cluster for Snapshot Isolation
        Sessions {3, 6, 9, 12, 15}
            
    -   AntidoteDB for Causal consistency
        Sessions {3, 6, 9, 12, 15}

3.  We considered two types of histories for each parameters for these databases.

    -   Uniformly randomized histories. Each operation is selected uniformly between `Read` and `Write` and each variable for that operation is chosen uniformly from the variable set.
    -   Disjoint writes. Each operation is selected uniformly as before. But all sessions are writing on disjoint sets of variables. The disjoint sets are equally distributed. The variable for each operation is chosen uniformly from its available variable set.

    The writes always happen with a new value, which we can maintain using a counter for each variable.

4.  For each cases of above,

    -   CockroachDB and Galera cluster
        50 uniformly randomized histories
        50 histories with disjoint write

    -   AntidoteDB
        100 uniformly randomized histories

    We did this to generate more consistent histories for CockroachDB and Galera cluster.

5.  We executed this histories on this databases and we log the executions.
6.  The executions are compressed in `executions.tar.bz`.
7.  Our tool can analyze the consistency of these executions.

### Instructions

0.  Requires

    -   [Rust](https://www.rust-lang.org/)
    -   [Python3](https://www.python.org/)
    -   Bash or similar shell.

1.  To generate the plots and data mentioned in the paper, clone `oopsla19` branch from our repository.
```
git clone --single-branch --branch oopsla19 https://gitlab.math.univ-paris-diderot.fr/ranadeep/dbcop.git
cd dbcop/oopsla19
bash run.sh setup
bash run.sh verify
bash run.sh plot
```
2.  `bash run.sh setup` decompresses the histories and create other necessary directories.
3.  `bash run.sh verify` verifies the histories and log the outputs from the tool.
4.  `bash run.sh plot` generates plots and numerical data from the verification logs. 
    `plots` directory stores the plots. It also outputs the number of minimal violation levels for each consistency levels for each case.
