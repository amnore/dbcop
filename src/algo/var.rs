#[derive(Debug, PartialEq, Eq)]
pub enum EventType {
    WRITE,
    READ,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Variable {
    pub id: u64,
    pub val: u64,
}

impl Variable {
    pub fn new(id: u64, val: u64) -> Self {
        Variable { id: id, val: val }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    pub ev_type: EventType,
    pub var: Variable,
}

impl Event {
    pub fn read(var: Variable) -> Self {
        Event {
            ev_type: EventType::READ,
            var: var,
        }
    }
    pub fn write(var: Variable) -> Self {
        Event {
            ev_type: EventType::WRITE,
            var: var,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Transaction {
    pub commit: bool,
    pub events: Vec<Event>,
}

pub fn create_txn() -> Transaction {
    let mut v = vec![];
    for i in 1..6 {
        v.push(Event {
            ev_type: EventType::READ,
            var: Variable { id: i, val: 10 },
        })
    }
    Transaction {
        commit: true,
        events: v,
    }
}
