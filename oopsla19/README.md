# OOPSLA '19 Artifact Submission

## Setting up the docker image

```
docker load -i paper_214.tar
mkdir -p plots
docker run --mount type=bind,source=`readlink -f plots`,target=/root/dbcop/oopsla19/plots -it --rm oopsla19
```

## Getting started

We implemented our work in a tool named `dbcop`(database cop). It can provides three things,

1.  Program generator to run on a database.
2.  A `trait`(equivalent to java interface) to run the programs on distributed databases and log the executions.
    An user can use this trait to write an implementation specific to a database.
3.  Consistency verifier for these executions.

`dbcop` offers two subcommands - `generate` and `verify`.

1.  `generate` generates programs to run on distributed databases.
2.  `verify` verifies consistency of the executions of these programs.

Running a generated programs on distributed databases requires advanced and complicated setup. So we collected the executions from 3 databases and focused on the performance of our verifier implementation. The collected executions are available in `/root/dbcop/oopsla19/execution` directory.

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

5.  We executed this histories on these databases and we log the executions.

### Instructions to reproduce

1.  `bash run.sh verify` generates the verifier logs.
2.  `bash run.sh plot` generates the plots and data.

#### A list of claims from the paper supported by the artifact

-   The plots in Figure 14 - scalability of our Serialization verifying implementation. They support our claim in section 6 (from line 1001) that our implementation scales really well even with varying number of sessions.
-   The plots in Figure 15a, 15b - scalability of our Snapshot isolation verifying implementation. They support our approach to implement the reduction from Snapshot Isolation verification to Serialization verification does not reduce performance.
-   The plot in Figure 15c - the performance of our Causal consistency verifying implementation. 
-   The minimum violation data in Table 2. They prove our claim in section 6 (from line 1016) that we found large number of violations in Galera cluster and CockroachDB.

#### A list of claims from the paper not supported by the artifact

We claimed SAT backend is much worse than our implementation in bigger histories. We did not include in the default script, because they may take very long time. Even with resource limit (`veri_stat.py:74-80`) of memory (10 gigabytes), created file size (10 gigabytes) and timeout (10 minutes).

1.  Just to verify the claim on few example someone can run our tool on a couple of big histories, and verify the running time with sat backend is indeed much bigger than with our default implementation.
    If it crashes your system, run with reduced resource limits mentioned in `veri_stat.py:30-33`.

    One example would be following. But one can choose any relatively bigger histories and verify the claim.
```
    dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si -c si
    dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si_sat -c si --sat
```
2.  If one wants to run on the all executions anyway, run `bash run.sh satverify` after `bash run.sh verify` and before `bash run.sh.plot`. Note, we reduced the resource limit to 2 minutes of timeout, 2 gigabytes of memory and 2 gigabytes for SAT cnf file for quicker runtime. But one can modify the parameters at `veri_stat.py:30-33` and try with larger resource limits.
