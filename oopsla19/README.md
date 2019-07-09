# OOPSLA'19 Artifact: Paper 214

## Setting up the Docker image

We provide a Docker image as an artifact. It is available at [Google Drive](https://drive.google.com). The required [Docker CE](https://docs.docker.com/install) version is 18.09.

```
bzcat -k paper_214.tar.bz | docker load
mkdir -p plots
docker run --mount type=bind,source=`readlink -f plots`,target=/root/dbcop/oopsla19/plots -it oopsla19
```

Plots will be available in `plots` directory after generating them in the Docker environment.

## Getting started

We implemented our work in a tool named [dbcop](https://gitlab.math.univ-paris-diderot.fr/ranadeep/dbcop) using [Rust-lang](https://www.rust-lang.org). It provides three things,

1.  A random generator for client programs to run on a database.
2.  A `trait`(equivalent to java interface) to run client programs on a database and log its executions. A user can use this trait to write an implementation specific to a database.
3.  A verifier that checks conformance of a given execution to a consistency model.

`dbcop` offers two subcommands: `generate` and `verify`.

1.  `generate` generates client programs to run on a database.
2.  `verify` verifies consistency of the executions of these client programs.

Logging enough executions (of client programs) is time-consuming (more than 10 hours) and requires a complicated setup. Therefore, our artifact includes the executions we have used to construct the plots included in the paper (Figure 14, 15 and Table 2). The collected executions are available in the folder  `/root/dbcop/oopsla19/executions`.

To get started, you can use the following to check causal consistency of an execution in AntidoteDB (this uses the algorithms we propose in our paper):
```
dbcop verify -d executions/antidote_all_writes/3_30_20_180/hist-00000 -o antidote_verifier_log -c cc
```
`-c` takes a consistency levels to check

1.  `cc` for Causal consistency
2.  `si` for Snapshot Isolation
3.  `ser` for Serialization

To verify consistency using a SAT solver (minisat), pass the `--sat` argument.
```
dbcop verify -d executions/antidote_all_writes/3_30_20_180/hist-00000 -o antidote_verifier_log -c cc --sat
```

Help instructions  `--help` are available for these commands.

## Step-by-step instructions

1.  We have generated executions for 3 databases.

    -   [CockroachDB](https://www.cockroachlabs.com/)
    -   [Galera cluster](https://galeracluster.com)
    -   [AntidoteDB](https://www.antidotedb.eu/)

2.  The execution generation process is parametrized by the number of sessions, transactions per session, operations per transaction, and maximal number of variables. We stored the executions in `'{}_{}_{}_{}'.format(n_sessions, n_transaction, n_operations, n_variables)` sub-directories for each combination of parameter values we report on.
    The history `executions/antidote_all_writes/3_30_20_180/hist-00000` verified above is an AntidoteDB execution with 3 sessions, 30 transactions per session, 20 operations per transaction, and 180 variables.

### Instructions to reproduce

1.  `bash run.sh verify` generates the verifier logs.
2.  `bash run.sh plot` generates the plots and data.

#### A list of claims from the paper supported by the artifact

-   `plots/roachdb_general_*.png` support the plots in Figure 14, the scalability of our Serializability verifying implementation. They support our claim in Section 6, line 1001, that our implementation scales when varying one of the parameters mentioned above (the wall clock times may differ).
-   `plots/{galera,roachdb}_sessions.png` support the plots in Figure 15a, 15b, the scalability of our Snapshot Isolation verifying implementation. 
-   `plots/antidote_sessions.png` supports the plot in Figure 15c, the performance of our Causal Consistency verifying implementation. 
-   Outputs from `bash run.sh plot` (data for `Galera` and `Roachdb_general`) supports the violation classification in Table 2 and the claims in Section 6, line 1016.

#### A list of claims from the paper not supported by the artifact

We claimed that our algorithm performs better than a best-effort reduction to SAT (this difference increases dramatically for longer executions). The support of this claim is not included in the default scripts, because applying the reduction to SAT on all executions takes a long time (more than 10 hours) even with limited resources (`veri_stat.py:74-80`) of memory (10 gigabytes), created file size (10 gigabytes) and timeout (10 minutes).

1.  To verify the claim on a few examples one can run our tool on a couple of long executions, and verify that the running time with a SAT backend is indeed much bigger than with our default implementation.

One example would be the following (any other longer execution will work):
```
dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si -c si
dbcop verify -d executions/roachdb_general_all_writes/15_30_20_900/hist-00009 -o roachdb_si_sat -c si --sat
```
2.  If one wants to run the SAT reduction on all executions and generate the complete plots in Figure 14,15, run `bash run.sh satverify` between `bash run.sh verify` and `bash run.sh.plot`.
    If it crashes the system or takes too long to complete, try lowering the resource limits in `veri_stat.py:30-33` before these commands.
