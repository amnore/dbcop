pub mod sat;
pub mod algo;
pub mod util;

#[derive(Debug)]
pub enum Consistency {
    RepeatableRead,
    ReadCommitted,
    Causal,
    Prefix,
    SnapshotIsolation,
    Serializable,
}
