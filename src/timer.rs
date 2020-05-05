use std::time::{Duration, SystemTime};

pub struct ScopedTimer {
    name: String,
    start: SystemTime,
    stop: Vec<(String, SystemTime)>,
}

impl ScopedTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: SystemTime::now(),
            stop: Vec::new(),
        }
    }

    pub fn checkpoint(&mut self, name: &str) {
        self.stop.push((name.to_string(), SystemTime::now()));
    }

    pub fn elapsed_since_checkpoint(&mut self) {
        let checkpoint = self.stop.pop().expect("No checkpoint");
        println!(
            "\t[{}] : {:?} msec",
            checkpoint.0,
            SystemTime::now()
                .duration_since(checkpoint.1)
                .unwrap()
                .as_millis(),
        )
    }

    pub fn elapsed(&self) {
        println!(
            "\t[{}] : {:?} msec",
            self.name,
            SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
        );
    }
}

impl std::ops::Drop for ScopedTimer {
    fn drop(&mut self) {
        self.elapsed();
    }
}
