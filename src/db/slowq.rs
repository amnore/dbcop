use mysql;
use mysql::time::{Duration, Timespec};
use algo::var::MySQLDur;

#[derive(Debug, PartialEq, Eq)]
struct LogRow {
    start_time: Option<Timespec>,
    user_host: Option<String>,
    query_time: Option<Duration>,
    lock_time: Option<Duration>,
    rows_sent: Option<i64>,
    rows_examined: Option<i64>,
    db: Option<String>,
    last_insert_id: Option<i64>,
    insert_id: Option<i64>,
    server_id: Option<i64>,
    sql_text: Option<String>,
    thread_id: Option<i64>,
    rows_affected: Option<i64>,
}

pub fn turn_on_slow_query(conn: &mut mysql::PooledConn) {
    conn.query("SET GLOBAL slow_query_log = 'ON'").unwrap();
    conn.query("SET GLOBAL long_query_time = 0").unwrap();
    conn.query("SET GLOBAL log_output = 'TABLE'").unwrap();
}

pub fn clean_slow_query(conn: &mut mysql::PooledConn) {
    conn.query("TRUNCATE TABLE mysql.slow_log").unwrap();
}

pub fn increase_max_connections(n_conn: u64, conn: &mut mysql::PooledConn) {
    conn.query(format!("SET GLOBAL max_connections = {}", n_conn))
        .unwrap();
}

fn get_slow_query(conn: &mut mysql::PooledConn) {
    let slow_log: Vec<_> = conn.prep_exec("SELECT * FROM mysql.slow_log WHERE db=?", ("test",))
        .map(|result| {
            result
                .map(|x| x.unwrap())
                .map(|mut row| LogRow {
                    start_time: row.take("start_time"),
                    user_host: row.take("user_host"),
                    query_time: row.take("query_time"),
                    lock_time: row.take("lock_time"),
                    rows_sent: row.take("rows_sent"),
                    rows_examined: row.take("rows_examined"),
                    db: row.take("db"),
                    last_insert_id: row.take("last_insert_id"),
                    insert_id: row.take("insert_id"),
                    server_id: row.take("server_id"),
                    sql_text: row.take("sql_text"),
                    thread_id: row.take("thread_id"),
                    rows_affected: row.take("rows_affected"),
                })
                .collect()
        })
        .unwrap();

    for ref e in slow_log.iter() {
        println!(
            "{:?} {:?} {:?} {:?} {:?}",
            e.start_time.unwrap(),
            e.query_time,
            e.lock_time,
            e.thread_id,
            e.sql_text
        );
    }
}

pub fn get_start_txn_durations(conn_id: u64, conn: &mut mysql::PooledConn) -> MySQLDur {
    let slow_log = conn.prep_exec(
        "SELECT * FROM mysql.slow_log WHERE thread_id=? AND sql_text LIKE 'START TRANSACTION'",
        (conn_id,),
    ).map(|result| {
            result
                .map(|x| x.unwrap())
                .map(|mut row| LogRow {
                    start_time: row.take("start_time"),
                    user_host: row.take("user_host"),
                    query_time: row.take("query_time"),
                    lock_time: row.take("lock_time"),
                    rows_sent: row.take("rows_sent"),
                    rows_examined: row.take("rows_examined"),
                    db: row.take("db"),
                    last_insert_id: row.take("last_insert_id"),
                    insert_id: row.take("insert_id"),
                    server_id: row.take("server_id"),
                    sql_text: row.take("sql_text"),
                    thread_id: row.take("thread_id"),
                    rows_affected: row.take("rows_affected"),
                })
                .collect::<Vec<_>>()
        })
        .unwrap();

    let start_time = slow_log[0].start_time.unwrap();
    let lock_time = slow_log[0].lock_time.unwrap();
    let query_time = slow_log[0].query_time.unwrap();
    MySQLDur {
        start_time: start_time,
        lock_time: start_time + lock_time,
        query_time: start_time + lock_time + query_time,
    }
}

pub fn get_end_txn_durations(conn_id: u64, conn: &mut mysql::PooledConn) -> MySQLDur {
    let slow_log = conn.prep_exec(
        "SELECT * FROM mysql.slow_log WHERE thread_id=? AND (sql_text LIKE 'COMMIT' OR sql_text LIKE 'ROLLBACK')",
        (conn_id,),
    ).map(|result| {
            result
                .map(|x| x.unwrap())
                .map(|mut row| LogRow {
                    start_time: row.take("start_time"),
                    user_host: row.take("user_host"),
                    query_time: row.take("query_time"),
                    lock_time: row.take("lock_time"),
                    rows_sent: row.take("rows_sent"),
                    rows_examined: row.take("rows_examined"),
                    db: row.take("db"),
                    last_insert_id: row.take("last_insert_id"),
                    insert_id: row.take("insert_id"),
                    server_id: row.take("server_id"),
                    sql_text: row.take("sql_text"),
                    thread_id: row.take("thread_id"),
                    rows_affected: row.take("rows_affected"),
                })
                .collect::<Vec<_>>()
        })
        .unwrap();

    let start_time = slow_log[0].start_time.unwrap();
    let lock_time = slow_log[0].lock_time.unwrap();
    let query_time = slow_log[0].query_time.unwrap();
    MySQLDur {
        start_time: start_time,
        lock_time: start_time + lock_time,
        query_time: start_time + lock_time + query_time,
    }
}

pub fn get_access_durations(conn_id: u64, conn: &mut mysql::PooledConn) -> Vec<MySQLDur> {
    let slow_log = conn.prep_exec(
        "SELECT * FROM mysql.slow_log WHERE thread_id=? AND (sql_text LIKE 'SELECT%' OR sql_text LIKE 'UPDATE%')",
        (conn_id,),
    ).map(|result| {
            result
                .map(|x| x.unwrap())
                .map(|mut row| LogRow {
                    start_time: row.take("start_time"),
                    user_host: row.take("user_host"),
                    query_time: row.take("query_time"),
                    lock_time: row.take("lock_time"),
                    rows_sent: row.take("rows_sent"),
                    rows_examined: row.take("rows_examined"),
                    db: row.take("db"),
                    last_insert_id: row.take("last_insert_id"),
                    insert_id: row.take("insert_id"),
                    server_id: row.take("server_id"),
                    sql_text: row.take("sql_text"),
                    thread_id: row.take("thread_id"),
                    rows_affected: row.take("rows_affected"),
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    slow_log
        .iter()
        .map(|x| {
            let start_time = x.start_time.unwrap();
            let lock_time = x.lock_time.unwrap();
            let query_time = x.query_time.unwrap();
            MySQLDur {
                start_time: start_time,
                lock_time: start_time + lock_time,
                query_time: start_time + lock_time + query_time,
            }
        })
        .collect()
}
