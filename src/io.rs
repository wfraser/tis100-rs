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
            self.pos += 1;
            Some((Port::ANY, self.values[self.pos - 1]))
        } else {
            None
        }
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
            if let Some((_port, val)) = avail_read {
                let received = val.take();
                if received == Some(self.values[self.pos]) {
                    self.pos += 1;
                    if self.pos == self.values.len() {
                        VerifyState::Finished
                    } else {
                        VerifyState::Okay
                    }
                } else {
                    println!("wrong input");
                    for n in &self.values {
                        print!("{} ", n);
                    }
                    println!();
                    for (idx, n) in self.values.iter().enumerate() {
                        if idx == self.pos {
                            println!("^");
                        } else {
                            for _ in 0 .. format!("{} ", n).len() {
                                print!(" ");
                            }
                        }
                    }
                    println!();
                    println!("got {} instead", received.unwrap());
                    VerifyState::Failed
                }
            } else {
                VerifyState::Blocked
            }
        } else {
            VerifyState::Finished
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum VerifyState {
    Failed,
    Okay,
    Blocked,
    Finished,
}
