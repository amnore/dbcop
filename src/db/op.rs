extern crate mysql;

use algo::var::{EventType, Transaction, Variable};

pub fn create_table(conn: &mut mysql::PooledConn) {
    conn.query("CREATE DATABASE IF NOT EXISTS dbcop").unwrap();
    conn.query(
        "CREATE TABLE IF NOT EXISTS dbcop.variables (id BIGINT(64) UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY, val BIGINT(64) UNSIGNED NOT NULL)",
    ).unwrap();
    conn.query("TRUNCATE TABLE dbcop.variables").unwrap();
    conn.query("USE dbcop").unwrap();
}

pub fn create_vars(limit: u64, conn: &mut mysql::PooledConn) {
    for mut stmt in conn.prepare("INSERT INTO dbcop.variables (val) values (?)")
        .into_iter()
    {
        for _ in 0..limit {
            stmt.execute((0,)).unwrap();
        }
    }
}

pub fn drop_database(conn: &mut mysql::PooledConn) {
    conn.query("DROP DATABASE dbcop").unwrap();
}

pub fn write_var(var: u64, val: u64, conn: &mut mysql::PooledConn) {
    for mut stmt in conn.prepare("UPDATE dbcop.variables SET val=? WHERE id=?")
        .into_iter()
    {
        stmt.execute((val, var)).unwrap();
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
    for mut sqltxn in conn.start_transaction(false, None, None) {
        for ref mut e in txn.events.iter_mut() {
            if e.ev_type == EventType::WRITE {
                sqltxn
                    .prep_exec(
                        "UPDATE dbcop.variables SET val=? WHERE id=?",
                        (e.var.val, e.var.id),
                    )
                    .unwrap();
            } else if e.ev_type == EventType::READ {
                sqltxn
                    .prep_exec("SELECT * FROM dbcop.variables WHERE id=?", (e.var.id,))
                    .and_then(|mut rows| {
                        let mut row = rows.next().unwrap().unwrap();
                        // assert_eq!(e.var.id, row.take::<u64, &str>("id").unwrap());
                        e.var.val = row.take("val").unwrap();
                        Ok(())
                    })
                    .unwrap();
            }
        }
        if txn.commit {
            sqltxn.commit().unwrap();
        } else {
            sqltxn.rollback().unwrap();
        }
    }
}
