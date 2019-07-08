#!/bin/bash

set -e

export DBCOP_INP_DIR="executions"
export DBCOP_OUT_DIR="verifications"
export PLOTS_DIR="plots"


verify() {
  mkdir -p ${DBCOP_OUT_DIR}

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/antidote_all_writes --out ${DBCOP_OUT_DIR}/antidote_all_writes --consistency cc --tag antidote_cc > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/antidote_all_writes --out ${DBCOP_OUT_DIR}/antidote_all_writes_inc --tag antidote_cc > /dev/null

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/galera_all_writes --out ${DBCOP_OUT_DIR}/galera_all_writes --consistency si --tag galera_all_si > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/galera_all_writes --out ${DBCOP_OUT_DIR}/galera_all_writes_inc --tag galera_all_si > /dev/null

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/galera_partition_writes --out ${DBCOP_OUT_DIR}/galera_partition_writes --consistency si --tag galera_partition_si > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/galera_partition_writes --out ${DBCOP_OUT_DIR}/galera_partition_writes_inc --tag galera_partition_si > /dev/null

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_all_writes --out ${DBCOP_OUT_DIR}/roachdb_all_writes --consistency si --tag roachdb_all_si > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_all_writes --out ${DBCOP_OUT_DIR}/roachdb_all_writes_inc --tag roachdb_all_si > /dev/null

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_partition_writes --out ${DBCOP_OUT_DIR}/roachdb_partition_writes --consistency si --tag roachdb_partition_si > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_partition_writes --out ${DBCOP_OUT_DIR}/roachdb_partition_writes_inc --tag roachdb_partition_si > /dev/null


  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_general_all_writes --out ${DBCOP_OUT_DIR}/roachdb_general_all_writes --consistency ser --tag roachdb_general_all_ser > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_general_all_writes --out ${DBCOP_OUT_DIR}/roachdb_general_all_writes_inc --tag roachdb_general_all_ser > /dev/null

  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_general_partition_writes --out ${DBCOP_OUT_DIR}/roachdb_general_partition_writes --consistency ser --tag roachdb_general_partition_ser > /dev/null
  python3 veri_stat.py --inp ${DBCOP_INP_DIR}/roachdb_general_partition_writes --out ${DBCOP_OUT_DIR}/roachdb_general_partition_writes_inc --tag roachdb_general_partition_ser > /dev/null
}


plot() {
  mkdir -p ${PLOTS_DIR}

  python3 plot_final.py galera
  python3 plot_final.py antidote
  python3 plot_final.py roachdb
  python3 plot_final.py roachdb_general --all
}


clean() {
  [[ -d ${DBCOP_OUT_DIR} ]] && rm -r ${DBCOP_OUT_DIR}
  [[ -d ${PLOTS} ]] && rm -r ${PLOTS}
}


case $1 in
  v|verify)
    echo Verifying the executed histories.
    verify
    ;;
  p|plot)
    echo Generating the plots and stats.
    plot
    ;;
  c|clean)
    echo Cleaning up the directory.
    clean
    ;;
  *)
    echo Unknown flag
    ;;
esac
