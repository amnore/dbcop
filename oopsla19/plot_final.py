import matplotlib as mpl
mpl.use('Agg')

import numpy as np
import matplotlib.pyplot as plt
import sqlite3
import itertools
import subprocess
import json
from collections import defaultdict
import argparse
import os


dbcop_inp = os.environ["DBCOP_INP_DIR"]
dbcop_out = os.environ["DBCOP_OUT_DIR"]
plots_dir = os.environ["PLOTS_DIR"]

slider = [
    [3, 6, 9, 12, 15],
    [20, 30, 40, 50, 60],
    [10, 20, 30, 40, 50],
    [50, 60, 70, 80, 90]
]

default = [6, 30, 20, 60]

title = ["sessions", "transactions", "operations", "variables"]

parser = argparse.ArgumentParser()
parser.add_argument("dbname")
parser.add_argument("--all", action="store_true")

args = parser.parse_args()

dbname = args.dbname

levels = {
    "antidote": 3,
    "roachdb": 5,
    "galera": 5,
    "roachdb_general": 6,
}

level = levels[dbname]

print(dbname.capitalize())

sat_data = True

ALL_EXEC = "{}/{}_all_writes".format(dbcop_inp, dbname)
ALL_VERI = "{}/{}_all_writes".format(dbcop_out, dbname)
ALL_VERI_SAT = "{}/{}_all_writes_sat".format(dbcop_out, dbname)
if not os.path.isdir(ALL_VERI_SAT):
    ALL_VERI_SAT = "{}/{}_all_writes".format(dbcop_out, dbname)
    sat_data = False
ALL_VERI_VIO = "{}/{}_all_writes_inc".format(dbcop_out, dbname)

PART_EXEC = "{}/{}_partition_writes".format(dbcop_inp, dbname)
PART_VERI = "{}/{}_partition_writes".format(dbcop_out, dbname)
PART_VERI_SAT = "{}/{}_partition_writes_sat".format(dbcop_out, dbname)
if not os.path.isdir(PART_VERI_SAT):
    PART_VERI_SAT = "{}/{}_partition_writes".format(dbcop_out, dbname)
    sat_data = False
PART_VERI_VIO = "{}/{}_partition_writes_inc".format(dbcop_out, dbname)


def get_overhead(file_path):
    return float(subprocess.check_output("history_duration {}".format(file_path).split(), stderr=None))


def get_id_sql(id, sessions, transactions, events, variables, path):
    with sqlite3.connect(path) as conn:
        c = conn.cursor()
        c.execute('select * from dbcopRuntime where exec_id=? and nClient=? and nTransaction=? and nEvent=? and nVariable=? order by ROWID',
                  (id, sessions, transactions, events, variables))
        rows = list(c.fetchall())
        # if len(rows) != 1:
        #     print(rows)
        #     print(id, sessions, transactions,
        #           events, variables, path, len(rows))
        return rows[-1]


def get_id(id_, sessions, transactions, events, variables, part):
    id = "hist-{:05}".format(id_)
    if part:
        sat_path = PART_VERI_SAT
        algo_path = PART_VERI
        vio_path = PART_VERI_VIO
        exec_path = PART_EXEC
    else:
        sat_path = ALL_VERI_SAT
        algo_path = ALL_VERI
        vio_path = ALL_VERI_VIO
        exec_path = ALL_EXEC
    sat_info = get_id_sql(id, sessions, transactions,
                          events, variables, sat_path + "/stats.db")
    algo_info = get_id_sql(id, sessions, transactions,
                           events, variables, algo_path + "/stats.db")
    bincode_path = '{}/{}_{}_{}_{}/{}/history.bincode'.format(
        exec_path, sessions, transactions, events, variables, id)
    exec_duration = get_overhead(bincode_path)
    # vio_info = get_id_sql(id, sessions, transactions,
    #                       events, variables, vio_db)

    with open('{}/{}_{}_{}_{}/{}/result_log.json'.format(
            vio_path, sessions, transactions, events, variables, id)) as f:
        json_d = json.loads(f.readlines()[-1])
        min_violation = json_d["minViolation"]

    algo_timeout = True
    sat_timeout = True

    try:
        with open('{}/{}_{}_{}_{}/{}/result_log.json'.format(
                algo_path, sessions, transactions, events, variables, id)) as f:
            json_d = json.loads(f.readlines()[-1])
            json_d["duration"]
            algo_timeout = False
    except:
        pass

    try:
        with open('{}/{}_{}_{}_{}/{}/result_log.json'.format(
                sat_path, sessions, transactions, events, variables, id)) as f:
            json_d = json.loads(f.readlines()[-1])
            json_d["duration"]
            sat_timeout = False
    except:
        pass

    total_transactions = None

    with open('{}/{}_{}_{}_{}/{}/result_log.json'.format(
            vio_path, sessions, transactions, events, variables, id)) as f:
        for line in f.readlines():
            json_d = json.loads(line)
            if 'number of transactions' in json_d:
                total_transactions = int(json_d['number of transactions'])

    # exec_id, nClient, nTransaction, nEvent, nVariable, binaryDuration, algoDuration, timedOut, result, sat, rc, _, _ =

    algo_duration = algo_info[5]
    sat_duration = sat_info[5]

    return (min_violation, total_transactions, sat_duration, sat_timeout, algo_duration, algo_timeout, exec_duration)


algo_durs = []
exec_durs = []


consistency_levels = ['ReadCommitted', 'ReadAtomic', 'Causal',
                      'Prefix', 'SnapshotIsolation', 'Serializable', 'ok']


plt.rcParams.update({'font.size': 20})

part_d = defaultdict(int)
no_part_d = defaultdict(int)

for i in range(4 if args.all else 1):
    current = []
    for j in range(4):
        if i == j:
            current.append(slider[j])
        else:
            current.append([default[j]])
    var_sat = []
    dur_sat = []
    ver_sat = []

    var_algo = []
    dur_algo = []
    ver_algo = []

    timeout_transactions_part = []
    timeout_transactions_no_part = []

    for conf in itertools.product(*current):
        nClient, nTransaction, nEvent, nVariable = conf
        nVariable *= nClient
        # print(conf)
        for id in range(100 if dbname == "antidote" else 50):
            for part in [False] if dbname == "antidote" else [True, False]:
                min_violation, total_transactions, sat_duration, sat_timeout, algo_duration, algo_timeout, exec_duration = get_id(id, nClient, nTransaction,
                                                                                                                                  nEvent, nVariable, part)
                if min_violation == "RepeatableRead":
                    min_violation = "ReadAtomic"
                # if min_violation not in ['ReadAtomic', 'Causal', 'Prefix', 'SnapshotIsolation', 'Serializable', 'ok']:
                # print(min_violation)
                # continue
                algo_durs.append(algo_duration)
                exec_durs.append(exec_duration)
                if part:
                    part_d[min_violation] += 1
                else:
                    no_part_d[min_violation] += 1
                var_sat.append(conf[i])
                dur_sat.append(sat_duration)
                if sat_timeout:
                    ver_sat.append('b')
                    assert(total_transactions is not None)
                    if part:
                        timeout_transactions_part.append(total_transactions)
                    else:
                        timeout_transactions_no_part.append(total_transactions)
                elif min_violation in consistency_levels[level:]:
                    ver_sat.append('g')
                else:
                    ver_sat.append('r')

                var_algo.append(conf[i])
                dur_algo.append(algo_duration)
                if algo_timeout:
                    ver_algo.append('b')
                    assert(False)
                elif min_violation in consistency_levels[level:]:
                    ver_algo.append('g')
                else:
                    ver_algo.append('r')
    # print(np.min(dur_algo))
    # print(np.min(dur_sat))
    plt.xlabel("{}".format(title[i]))
    plt.ylabel("runtime(seconds)")
    # plt.title(title[i])
    plt.yscale('log')
    plt.scatter(var_algo, dur_algo, s=80, c=ver_algo, marker='o', alpha=0.2)
    if sat_data:
        plt.scatter(var_sat, dur_sat, s=80, c=ver_sat, marker='^', alpha=0.2)
    plt.tight_layout()
    # plt.legend(("algo", "sat"), loc='lower right')
    plt.savefig('{}/{}_{}.png'.format(plots_dir, dbname, title[i]))
    # plt.show()
    plt.clf()
    if len(timeout_transactions_part) > 0:
        print("parition: no. history timedout", len(timeout_transactions_part))
        print("parition: no. transaction for timeout",
              np.mean(timeout_transactions_part))
    if len(timeout_transactions_no_part) > 0:
        print("normal: no. history timedout",
              len(timeout_transactions_no_part))
        print("normal: no. transaction for timeout",
              np.mean(timeout_transactions_no_part))

if len(part_d) > 0:
    print("Disjoint Violations")
for e in consistency_levels:
    if part_d[e] != 0:
        print("\t", e, part_d[e])

if len(no_part_d) > 0:
    print("No-disjoint Violations")
for e in consistency_levels:
    if no_part_d[e] != 0:
        print("\t", e, no_part_d[e])
# print("Average Overhead(%) to run the verifier with an execution", np.sum(algo_durs) * 100 / np.sum(exec_durs))
print()
