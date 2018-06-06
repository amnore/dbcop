use algo::txn::Transaction;
use algo::var::{EventType, Variable};
use mysql;

pub fn create_table(conn: &mut mysql::PooledConn) {
    // drop_database(conn);
    conn.query("CREATE DATABASE IF NOT EXISTS dbcop").unwrap();
    conn.query("DROP TABLE dbcop.variables").unwrap();
    conn.query(
        "CREATE TABLE IF NOT EXISTS dbcop.variables (id BIGINT(64) UNSIGNED NOT NULL PRIMARY KEY, val BIGINT(64) UNSIGNED NOT NULL, wr_node BIGINT(64) UNSIGNED NOT NULL, wr_txn BIGINT(64) UNSIGNED NOT NULL, wr_pos BIGINT(64) UNSIGNED NOT NULL)",
    ).unwrap();
    // conn.query("TRUNCATE TABLE dbcop.variables").unwrap();
    conn.query("USE dbcop").unwrap();
}

pub fn create_vars(offset: usize, n_var: usize, conn: &mut mysql::PooledConn) {
    for mut stmt in conn.prepare(
        "INSERT INTO dbcop.variables (id, val, wr_node, wr_txn, wr_pos) values (?, ?, 0, 0, 0)",
    ).into_iter()
    {
        for v in offset..(offset + n_var) {
            stmt.execute((v, 0)).unwrap();
        }
    }
}

pub fn clean_table(conn: &mut mysql::PooledConn) {
    conn.query("UPDATE dbcop.variables SET val=0, wr_node=0, wr_txn=0, wr_pos=0")
        .unwrap();
}

pub fn drop_database(conn: &mut mysql::PooledConn) {
    conn.query("DROP DATABASE dbcop").unwrap();
}

pub fn write_var(var: u64, val: u64, action_id: (u64, u64, u64), conn: &mut mysql::PooledConn) {
    for mut stmt in conn.prepare(
        "UPDATE dbcop.variables SET val=?, wr_node=?, wr_txn=?, wr_pos=? WHERE id=?",
    ).into_iter()
    {
        stmt.execute((val, action_id.0, action_id.1, action_id.2, var))
            .unwrap();
    }
}

pub fn read_var(var: u64, conn: &mut mysql::PooledConn) -> Variable {
    conn.first_exec("SELECT * FROM dbcop.variables WHERE id=?", (var,))
        .map(|result| {
            let mut row: mysql::Row = result.unwrap();
            Variable {
                id: row.take("id").unwrap(),
                val: row.take("val").unwrap(),
            }
        })
        .unwrap()
}

pub fn get_connection_id(conn: &mut mysql::PooledConn) -> u64 {
    conn.first_exec("SELECT connection_id()", ())
        .map(|result| {
            let mut row: mysql::Row = result.unwrap();
            row.take("connection_id()").unwrap()
        })
        .unwrap()
}

pub fn do_transaction(txn: &mut Transaction, conn: &mut mysql::PooledConn) {
    /*
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
    */
    for mut sqltxn in conn.start_transaction(
        false,
        Some(mysql::IsolationLevel::Serializable),
        Some(false),
    ) {
        for ref mut e in txn.events.iter_mut() {
            if e.ev_type == EventType::WRITE {
                match sqltxn.prep_exec(
                    "UPDATE dbcop.variables SET val=? WHERE id=?",
                    (e.var.val, e.var.id),
                ) {
                    Err(e) => {
                        println!("WRITE ERR -- {:?}", e);
                    }
                    _ => {}
                }
            } else if e.ev_type == EventType::READ {
                match sqltxn
                    .prep_exec("SELECT * FROM dbcop.variables WHERE id=?", (e.var.id,))
                    .and_then(|mut rows| {
                        let mut row = rows.next().unwrap().unwrap();
                        // assert_eq!(e.var.id, row.take::<u64, &str>("id").unwrap());
                        e.var.val = row.take("val").unwrap();
                        Ok(())
                    }) {
                    Err(e) => {
                        println!("READ ERR -- {:?}", e);
                    }
                    _ => {}
                }
            }
        }
        if txn.commit {
            sqltxn.commit().unwrap();
        } else {
            sqltxn.rollback().unwrap();
        }
    }
}
