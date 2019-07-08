import subprocess
import argparse
import pathlib
import time
import sqlite3
import glob
import re
import psutil
import json
import os

parser = argparse.ArgumentParser()
parser.add_argument("--inp", help="bulk execution directory", required=True)
parser.add_argument("--out", help="bulk verifying directory", required=True)
parser.add_argument(
    "--consistency", help="consistency to check")
parser.add_argument("--sat", help="use minisat", action="store_true")
parser.add_argument("--tag", help="tag to refer later", required=True)
parser.add_argument("--comment", help="comment")

args = parser.parse_args()

exec_path = pathlib.Path(args.inp)
veri_path = pathlib.Path(args.out)

veri_path.mkdir(parents=True, exist_ok=True)

db_path = veri_path / "stats.db"

timeout_minutes = 10
MEMORY_LIMIT = 10 * 1024 ** 3  # 10 giga bytes
FSIZE_LIMIT = 10 * 1024 ** 3  # 10 giga bytes


def get_conf_from_id(st):
    pat = re.compile('(\d+)_(\d+)_(\d+)_(\d+)')
    nClient = pat.match(st).group(1)
    nTransaction = pat.match(st).group(2)
    nEvent = pat.match(st).group(3)
    nVariable = pat.match(st).group(4)
    return (int(nClient), int(nTransaction), int(nEvent), int(nVariable))


with sqlite3.connect(str(db_path)) as conn:
    c = conn.cursor()
    c.execute("CREATE TABLE IF NOT EXISTS dbcopRuntime(exec_id TEXT, nClient INT, nTransaction INT, nEvent INT, nVariable INT, binaryDuration REAL, algoDuration REAL, timedOut INT, result TEXT, sat INT, returnCode INT, comment TEXT, tag TEXT)")

    for e in glob.glob(str(exec_path / '*' / '*')):
        single_exec_path = pathlib.Path(e)

        conf_id = single_exec_path.parent.name

        nClient, nTransaction, nEvent, nVariable = get_conf_from_id(conf_id)
        exec_id, _ = os.path.splitext(single_exec_path.name)

        single_out_path = veri_path / conf_id / exec_id

        cmd = []

        cmd.append('dbcop')
        cmd.append('verify')
        cmd.extend(['-d', str(single_exec_path)])
        cmd.extend(['-o', str(single_out_path)])
        if args.consistency:
            cmd.extend(['--cons', args.consistency])

        if args.sat:
            cmd.append('--sat')
        start_time = time.time()

        proc = subprocess.Popen(cmd)
        child_process = psutil.Process(proc.pid)

        child_process.rlimit(
            psutil.RLIMIT_AS, (MEMORY_LIMIT, MEMORY_LIMIT))
        child_process.rlimit(psutil.RLIMIT_FSIZE,
                             (FSIZE_LIMIT, FSIZE_LIMIT))

        try:
            outs, errs = proc.communicate(timeout=60 * timeout_minutes)
            end_time = time.time()
            timedOut = False
        except subprocess.TimeoutExpired:
            proc.kill()
            outs, errs = proc.communicate()
            end_time = time.time()
            timedOut = True

        binaryDuration = end_time - start_time

        result = None
        algoDuration = -1
        sat = args.sat

        if not timedOut:
            try:
                with open(str(single_out_path / 'result_log.json')) as f:
                    json_d = json.loads(f.readlines()[-1])
                    result = json_d["result"]
                    algoDuration = json_d["duration"]
                    sat = json_d["sat"]
            except:
                pass

        rc = proc.returncode

        c.execute("INSERT INTO dbcopRuntime (exec_id, nClient, nTransaction, nEvent, nVariable, binaryDuration, algoDuration, timedOut, result, sat, returnCode, comment, tag) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                  (exec_id, nClient, nTransaction, nEvent, nVariable, binaryDuration, algoDuration, timedOut, result, sat, rc, args.comment, args.tag))

        conn.commit()
