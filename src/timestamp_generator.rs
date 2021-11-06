use once_cell::sync::OnceCell;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};

pub static TIMESTAMP_GENERATOR_GLOBAL: OnceCell<Box<dyn TimestampGenerator + Send + Sync>> =
    OnceCell::new();

/// A trait used for getting timestamps. The purpose of this is to allow the global TIMESTAMP_GENERATOR_GLOBAL to be mockable.
/// The default implementation simply wraps the system clock.
/// We require Debug so we can unwrap the Result from OnceCell.
pub trait TimestampGenerator: Debug {
    fn get_timestamp(&self) -> u64 {
        println!("GET REAL TIMESTAMP");
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    fn advance(&self, _time_difference: u64) {
        panic!("Real TimestampGenerator should never be advanced");
    }
}
/// This is simply a wrapper for the system clock which allows us to mock the clock. A global reference is kept to once so
/// that we dont' need to pass a timestamp to every function which requires a timestamp.
#[derive(Debug)]
pub struct SystemTimestampGenerator {}

impl SystemTimestampGenerator {
    pub fn new() -> Self {
        SystemTimestampGenerator {}
    }
}

impl TimestampGenerator for SystemTimestampGenerator {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utilities::init_globals_for_tests;

    #[tokio::test]
    async fn mock_timestamp_generator_test() {
        init_globals_for_tests();
        let first_timestamp = TIMESTAMP_GENERATOR_GLOBAL.get().unwrap().get_timestamp();
        TIMESTAMP_GENERATOR_GLOBAL.get().unwrap().advance(1000);
        assert_eq!(
            first_timestamp + 1000,
            TIMESTAMP_GENERATOR_GLOBAL.get().unwrap().get_timestamp()
        );
    }
}
