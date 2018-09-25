use ansi_term::Style;
use std::fmt;

pub struct Event {
    pub id: usize,
    pub write: bool,
    pub variable: usize,
    pub value: usize,
    pub success: bool,
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let repr = format!(
            "[{}|{}({}):{}]",
            self.id,
            if self.write { 'W' } else { 'R' },
            self.variable,
            self.value
        );
        // write!(
        //     f,
        //     "{}",
        //     if self.success {
        //         repr
        //     } else {
        //         format!("{}", Style::new().strikethrough().paint(repr))
        //     }
        // )
        if !self.success {
            write!(f, "!");
        }
        write!(f, "{}", repr)
    }
}

impl Event {
    pub fn read(id: usize, var: usize) -> Self {
        Event {
            id: id,
            write: false,
            variable: var,
            value: 0,
            success: false,
        }
    }
    pub fn write(id: usize, var: usize, val: usize) -> Self {
        Event {
            id: id,
            write: true,
            variable: var,
            value: val,
            success: false,
        }
    }
}

pub struct Transaction {
    pub events: Vec<Event>,
    pub success: bool,
}

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let repr = format!("{:?}", self.events);
        // write!(
        //     f,
        //     "{}",
        //     if self.success {
        //         repr
        //     } else {
        //         format!("{}", Style::new().strikethrough().paint(repr))
        //     }
        // )
        if !self.success {
            write!(f, "!");
        }
        write!(f, "{}", repr)
    }
}
