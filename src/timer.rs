use std::time::{Duration, Instant};

pub struct Timer {
    time: Duration,
    mark: Instant,
    running: bool,
}

impl Timer {
    pub fn new(time: Duration, running: bool) -> Timer {
        Timer {
            time,
            mark: Instant::now(),
            running
        }
    }
    pub fn time(&self) -> Duration {
        if self.running {
            self.time.saturating_sub(
                Instant::now().duration_since(self.mark)
            )
        } else {
            self.time
        }
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn resume(&mut self) {
        if !self.running {
            self.mark = Instant::now();
            self.running = true;
        }
    }
    pub fn pause(&mut self) {
        if self.running {
            self.time = self.time();
            self.running = false;
        }
    }
    pub fn add(&mut self, time: Duration) {
        self.time += time;
    }
}
