# OOPSLA '19 Artifact Submission

## Setting up the docker image

Required [Docker CE](https://docs.docker.com/install) version is 18.09.

```
docker load -i paper_214.tar
mkdir -p plots
docker run --mount type=bind,source=`readlink -f plots`,target=/root/dbcop/oopsla19/plots -it --rm oopsla19
```

Plots will be available in `plots` directory after generating them in Docker environment.

## Getting started

We implemented our work in a tool named [dbcop](https://gitlab.math.univ-paris-diderot.fr/ranadeep/dbcop) using [Rust-lang](https://www.rust-lang.org). It provides three things,

1.  Program generator to run on a database.
2.  A `trait`(equivalent to java interface) to run the programs on distributed databases and log the executions.
    A user can use this trait to write an implementation specific to a database.
3.  Consistency verifier for these executions.

`dbcop` offers two subcommands - `generate` and `verify`.

1.  `generate` generates programs to run on distributed databases.
2.  `verify` verifies consistency of the executions of these programs.

Running a generated program on distributed databases requires an advanced and complicated setup. So we collected the executions from 3 databases and focused on the performance of our verifier implementation. The collected executions are available in `/root/dbcop/oopsla19/execution` directory.

To get started, let's verify causal consistency of an execution in AntidoteDB. 
```
dbcop verify -d executions/antidote_all_writes/3_30_20_180/hist-00000 -o antidote_verifier_log -c cc
```
`-c` takes a consistency levels to check

1.  `cc` for Causal consistency
2.  `si` for Snapshot Isolation
3.  `ser` for Serialization

To use Sat solver(minisat) backend, pass `--sat` argument.
```
dbcop verify -d executions/antidote_all_writes/3_30_20_180/hist-00000 -o antidote_verifier_log -c cc --sat
```
This is will verify the history with sat solver.

There are `--help` available for these commands.

## Step-by-step instructions

1.  We have generated histories for 3 databases.

    -   [CockroachDB](https://www.cockroachlabs.com/)
    -   [Galera cluster](https://galeracluster.com)
    -   [AntidoteDB](https://www.antidotedb.eu/)

2.  We used 6 sessions, 30 transactions per sessions, 20 operations per transactions and 60 average variables per session as default and varied one fixed parameter.
    We stored the executions in `'{}_{}_{}_{}'.format(n_sessions, n_transaction, n_operations, n_variables)` sub-directories for each cases.
    So when we verified the history `executions/antidote_all_writes/3_30_20_180/hist-00000`, we verified an AntidoteDB execution with 3 sessions, 30 transactions per sessions, 20 operations per transactions and 180 variables.

3.  We considered two types of histories for each parameter for these databases.

    -   Uniformly randomized histories. Each operation is selected uniformly between `Read` and `Write` and each variable for that operation is chosen uniformly from the variable set.
    -   Disjoint writes. Each operation is selected uniformly as before. But all sessions are writing on disjoint sets of variables. The disjoint sets are equally distributed. The variable for each operation is chosen uniformly from its available variable set.

    The writes always happen with a new value, which we can maintain using a counter for each variable.

4.  For each case of above,

    -   CockroachDB and Galera cluster
        50 uniformly randomized histories
        50 histories with disjoint write

    -   AntidoteDB
        100 uniformly randomized histories

    We did this to generate more consistent histories for CockroachDB and Galera cluster.

5.  We executed these histories on these databases and we log the executions.

### Instructions to reproduce

1.  `bash run.sh verify` generates the verifier logs.
2.  `bash run.sh plot` generates the plots and data.

#### A list of claims from the paper supported by the artifact

-   `plots/roachdb_general_*.png` support the plots in Figure 14, the scalability of our Serialization verifying implementation. They support our claim in section 6 (from line 1001) that our implementation scales really well even with a varying number of sessions.
-   `plots/{galera,roachdb}_sessions.png` support the plots in Figure 15a, 15b, the scalability of our Snapshot isolation verifying implementation. They support our implementation of the reduction from Snapshot Isolation verification to Serialization verification does not reduce performance.
-   `plots/antidote_sessions.png` supports the plot in Figure 15c, the performance of our Causal consistency verifying implementation. 
-   Outputs from `bash run.sh plot` (data for `Galera` and `Roachdb_general`) supports the minimum violation data in Table 2. They prove our claim in section 6 (from line 1016) that we found a large number of violations in Galera cluster and CockroachDB.

#### A list of claims from the paper not supported by the artifact

We claimed the performance of SAT backend is much worse than our implementation in bigger histories. We did not include it in the default script, because they may take a very long time for large histories even with a limited resource (`veri_stat.py:74-80`) of memory (10 gigabytes), created file size (10 gigabytes) and timeout (10 minutes).

1.  Just to verify the claim on a few examples one can run our tool on a couple of big histories, and verify the running time with sat backend is indeed much bigger than with our default implementation.

    One example would be the following. But one can choose any reasonable bigger histories and verify the claim.
```
dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si -c si
dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si_sat -c si --sat
```
2.  If one wants to run on the all executions to generate the plots, run `bash run.sh satverify` between `bash run.sh verify` and `bash run.sh.plot`.
    If it crashes your system, run with reduced resource limits mentioned in `veri_stat.py:30-33`.
