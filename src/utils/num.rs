pub fn bool_to_f32(x: bool) -> f32 {
    if x {
        1.0
    } else {
        0.0
    }
}

pub fn y_to_z(y: f32) -> f32 {
    100.0 - y / 10000.0
}

use std::collections::VecDeque;

pub struct MyQueue {
    queue: VecDeque<f32>,
    max_size: usize,
}

impl MyQueue {
    pub fn new(max_size: usize) -> Self {
        MyQueue {
            queue: VecDeque::new(),
            max_size,
        }
    }
    pub fn add(&mut self, item: f32) {
        if self.queue.len() == self.max_size {
            self.queue.pop_front();
        }
        self.queue.push_back(item);
    }

    pub fn iSIncreased(&mut self) -> bool {
        let len = self.queue.len();
        let maxNum = self
            .queue
            .iter()
            .reduce(|accum, item| if accum >= item { accum } else { item });

        let latestNum = self.queue.get(len - 1);
        if maxNum == latestNum {
            return true;
        } else {
            return false;
        }
    }
}
