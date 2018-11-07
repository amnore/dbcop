# DBCop

### __For recent work, look at `wip` branch.__

## Usage

1.  Clone it.
```
    git clone git@gitlab.math.univ-paris-diderot.fr:ranadeep/dbcop.git
```

2.  Compile and install using `cargo` and run.
    Make sure `~/.cargo/bin` is in your system path and MySQL server is running on `localhost` at `3306` port.
```
    cd dbcop
    cargo install
    ./dbcop -u root -p <root_password>
```
or you can simply do after changing directory,
```
    cargo run -- -u root -p <root_password>
```

3.  Slow query log will be available at `mysql.slow_log` table.

4.  You can fetch the queries of a particular thread using,
```
    SELECT * FROM mysql.slow_log WHERE thread_id = ?
```
Replace `?` with the number corresponding to a thread from `dbcop` output.
