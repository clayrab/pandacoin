use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};

/// A trait used for getting timestamps. The purpose of this is to allow the timestamp generator to be mockable.
/// The default implementation simply wraps the system clock.
pub trait AbstractTimestampGenerator {
    fn get_timestamp(&self) -> u64 {
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

impl AbstractTimestampGenerator for SystemTimestampGenerator {}

#[cfg(test)]
mod test {
    use crate::test_utilities::globals_init::make_timestamp_generator_for_test;

    #[tokio::test]
    async fn mock_timestamp_generator_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let first_timestamp = timestamp_generator.get_timestamp();
        timestamp_generator.advance(1000);
        assert_eq!(first_timestamp + 1000, timestamp_generator.get_timestamp());
    }
}
