use crate::arch::read_clock_counter;

pub struct StopWatch {
    current: u64,
}

impl StopWatch {
    pub fn start() -> StopWatch {
        let current = read_clock_counter();
        StopWatch { current }
    }

    pub fn lap_time(&mut self, title: &'static str) {
        let current = read_clock_counter();
        trace!("profiler: {} counts ({})", current - self.current, title,);
        self.current = current;
    }
}
