use crate::instr::Port;

#[derive(Debug)]
pub struct InputNode {
    values: Vec<i32>,
    pos: usize,
}

impl InputNode {
    pub fn new(values: Vec<i32>) -> Self {
        InputNode {
            values,
            pos: 0,
        }
    }

    pub fn write(&mut self) -> Option<(Port, i32)> {
        if self.pos < self.values.len() {
            let value = self.values[self.pos];
            trace!("writing {}", value);
            info!("{}", self);
            self.pos += 1;
            Some((Port::ANY, value))
        } else {
            trace!("no more values");
            None
        }
    }
}

impl std::fmt::Display for InputNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut out = String::new();
        for (idx, n) in self.values.iter().enumerate() {
            if idx == self.pos {
                out += &format!("<({})> ", n);
            } else {
                out += &format!("{} ", n);
            }
        }
        out.pop();
        f.pad(&out)
    }
}

#[derive(Debug)]
pub struct OutputNode {
    values: Vec<i32>,
    pos: usize,
    verified: VerifyState,
}

impl OutputNode {
    pub fn new(values: Vec<i32>) -> Self {
        OutputNode {
            values,
            pos: 0,
            verified: VerifyState::Blocked,
        }
    }

    pub fn read(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> Option<Port> {
        let state = self.do_verify(&mut avail_reads.get_mut(0));
        self.verified = state;
        match state {
            VerifyState::Okay => None,
            VerifyState::Blocked => Some(Port::ANY),
            VerifyState::Finished | VerifyState::Failed => None,
        }
    }

    pub fn verified(&self) -> VerifyState {
        self.verified
    }

    pub fn do_verify(&mut self, avail_read: &mut Option<&mut (Port, Option<i32>)>) -> VerifyState {
        if self.pos < self.values.len() {
            if let Some((port, val)) = avail_read {
                let received = val.take().unwrap();
                info!("checking value {} from {}", received, port);
                if received == self.values[self.pos] {
                    self.pos += 1;
                    info!("{}", self);
                    if self.pos == self.values.len() {
                        info!("finished now!");
                        VerifyState::Finished
                    } else {
                        info!("value is correct");
                        VerifyState::Okay
                    }
                } else {
                    error!("wrong input");
                    error!("{}", self);
                    error!("got {} instead", received);
                    VerifyState::Failed
                }
            } else {
                trace!("waiting for input");
                VerifyState::Blocked
            }
        } else {
            trace!("finished");
            VerifyState::Finished
        }
    }
}

impl std::fmt::Display for OutputNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut out = String::new();
        for (idx, n) in self.values.iter().enumerate() {
            if idx == self.pos {
                out += &format!("<({})> ", n);
            } else {
                out += &format!("{} ", n);
            }
        }
        out.pop();
        f.pad(&out)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum VerifyState {
    Failed,
    Okay,
    Blocked,
    Finished,
}
