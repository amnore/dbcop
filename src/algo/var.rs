use std::fmt;
use mysql::time::Timespec;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum EventType {
    WRITE,
    READ,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MySQLDur {
    pub start_time: Timespec,
    pub lock_time: Timespec,
    pub query_time: Timespec,
}

impl MySQLDur {
    pub fn new() -> Self {
        MySQLDur {
            start_time: Timespec::new(0, 0),
            lock_time: Timespec::new(0, 0),
            query_time: Timespec::new(0, 0),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Variable {
    pub id: usize,
    pub val: (usize, usize, usize, usize)
}

impl Variable {
    pub fn new(id: usize, val: (usize, usize, usize, usize)) -> Self {
        Variable { id: id, val: val }
    }

    pub fn is_zero(&self) -> bool {
        self.val.0 == 0
    }
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{:?}", self.id, self.val)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Event {
    pub ev_type: EventType,
    pub var: Variable,
    pub dur: MySQLDur,
}

impl Event {
    pub fn read(var: Variable) -> Self {
        Event {
            ev_type: EventType::READ,
            var: var,
            dur: MySQLDur::new(),
        }
    }
    pub fn write(var: Variable) -> Self {
        Event {
            ev_type: EventType::WRITE,
            var: var,
            dur: MySQLDur::new(),
        }
    }

    pub fn is_write(&self) -> bool {
        self.ev_type == EventType::WRITE
    }

    pub fn is_read(&self) -> bool {
        self.ev_type == EventType::READ
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.ev_type {
            EventType::READ => write!(f, "{}({:?})", 'R', self.var),
            EventType::WRITE => write!(f, "{}({:?})", 'W', self.var),
        }
    }
}
