extern crate mysql;

use mysql::time::Timespec;
use mysql::time::Duration;

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
