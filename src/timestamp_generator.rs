use std::{time::{SystemTime, UNIX_EPOCH}};

lazy_static! {
    pub static ref TIMESTAMP_GENERATOR_GLOBAL: Box<dyn TimestampGenerator + Send + Sync> = Box::new(SystemTimestampGenerator::new());
}
/// A trait used for getting timestamps. The purpose of this is to allow the global TIMESTAMP_GENERATOR_GLOBAL to be mockable.
/// The default implementation simply wraps the system clock.
pub trait TimestampGenerator {
    fn get_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    fn advance(&self, _time_difference: u64) {
        println!("TimestampGenerator advance");
        // noop
    }
}
/// This is simply a wrapper for the system clock which allows us to mock the clock. A global reference is kept to once so
/// that we dont' need to pass a timestamp to every function which requires a timestamp.
pub struct SystemTimestampGenerator {}
impl SystemTimestampGenerator {
    pub fn new() -> Self {
        SystemTimestampGenerator {}
    }
}
impl TimestampGenerator for SystemTimestampGenerator {

}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utilities::mock_timestamp_generator::MockTimestampGenerator;

    lazy_static! {
        pub static ref TIMESTAMP_GENERATOR_GLOBAL: Box<dyn TimestampGenerator + Send + Sync> = Box::new(MockTimestampGenerator::new());
    }

    #[tokio::test]
    async fn mock_timestamp_generator_test() {
        let first_timestamp = TIMESTAMP_GENERATOR_GLOBAL.get_timestamp();
        TIMESTAMP_GENERATOR_GLOBAL.advance(1000);
        assert_eq!(first_timestamp + 1000, TIMESTAMP_GENERATOR_GLOBAL.get_timestamp());        
    }
}
