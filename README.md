# DBCop

## Usage

1.  Clone it.
```
    git clone -b wip git@gitlab.math.univ-paris-diderot.fr:ranadeep/dbcop.git
```

2. Fire up Galera cluster using  `docker-compose`.
```
cd docker
sudo docker-compose up
```

3.  Compile and install using `cargo` and run.
    Make sure `~/.cargo/bin` is in your system path.
```
    cd dbcop
    cargo install
    dbcop
```
