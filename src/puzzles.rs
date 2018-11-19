use crate::instr::Port;
use rand::Rng;
use std::collections::BTreeMap;

pub const PUZZLE_WIDTH: usize = 4;
pub const PUZZLE_HEIGHT: usize = 3;

#[derive(Debug)]
pub struct Puzzle {
    pub name: &'static str,
    pub bad_nodes: &'static [usize],
    pub inputs: BTreeMap<(usize, Port), Vec<i32>>,
    pub outputs: BTreeMap<(usize, Port), Vec<i32>>,
}

//pub type InputGenerator = Fn(&mut dyn rand::Rng) -> i32;
//pub type InputGenerator = Iterator<Item=i32>;
//pub type InputGenerators = BTreeMap<(NodeId, Port), Box<Iterator<Item=i32> + 'static>>;

/*
struct RngGenerator<R, F> {
    rng: R,
    f: F,
    past: Vec<i32>,
}

impl<R: Rng, F: 'static + Fn(&[i32], &mut R) -> i32> Iterator for RngGenerator<R, F> {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        let next = (self.f)(&self.past, &mut self.rng);
        self.past.push(next);
        Some(next)
    }
}

impl<R: Rng, F: Fn(&[i32], &mut R) -> i32> RngGenerator<R, F> {
    pub fn new(rng: R, f: F) -> Self {
        Self { rng, f, past: vec![] }
    }
}
*/

fn random_vec(rng: &mut impl Rng, num: usize, min: i32, max: i32) -> Vec<i32> {
    let range = rand::distributions::Uniform::new_inclusive(min, max);
    rng.sample_iter(&range).take(num).collect()
}

pub fn get_puzzle<R: Rng + Clone + 'static>(number: &str, input_size: usize, mut rng: R)
    -> Option<Puzzle>
{
    Some(match number {
        "DBG01" => {
            Puzzle {
                name: "[simulator debug] Connectivity Check",
                bad_nodes: &[],
                inputs: btreemap! {
                    (1, Port::UP) => vec![1,2,3,4],
                },
                outputs: btreemap! {
                    (10, Port::DOWN) => vec![10,20,30,40],
                },
            }
        }
        "00150" => {
            let r1 = random_vec(&mut rng, input_size, 10, 100);
            //let r1 = vec![51,62,16,83,61,14,35];
            let r2 = random_vec(&mut rng, input_size, 10, 100);
            //let r2 = vec![68,59,59,49,82,16,45];
            Puzzle {
                name: "Self-Test Diagnostic",
                bad_nodes: &[1, 5, 7, 9],
                inputs: btreemap! {
                    (0, Port::UP) => r1.clone(),
                    (3, Port::UP) => r2.clone(),
                },
                outputs: btreemap! {
                    (8, Port::DOWN) => r1,
                    (11, Port::DOWN) => r2,
                },
            }
        }
        "41427" => {
            let mut input = vec![];
            let mut output1 = vec![999];
            let mut output2 = vec![0];
            for i in 0 .. input_size {
                if i > 0
                    && input.last() != Some(&0)
                    && (i == input_size - 1
                        || 0 == rng.gen_range(0, 5))
                {
                    input.push(0);
                    if i != input_size - 1 {
                        output1.push(999);
                        output2.push(0);
                    }
                } else {
                    let value = rng.gen_range(10, 100);
                    input.push(value);
                    if Some(&value) < output1.last() {
                        *output1.last_mut().unwrap() = value;
                    }
                    if Some(&value) > output2.last() {
                        *output2.last_mut().unwrap() = value;
                    }
                }
            }

            Puzzle {
                name: "Sequence Peak Detector",
                bad_nodes: &[7],
                inputs: btreemap! {
                    (1, Port::UP) => input,
                },
                outputs: btreemap! {
                    ( 9, Port::DOWN) => output1,
                    (10, Port::DOWN) => output2,
                },
            }
        },
        _ => return None
    })
}
