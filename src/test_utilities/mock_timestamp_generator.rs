use crate::timestamp_generator::AbstractTimestampGenerator;
use std::fmt::Debug;
use std::{
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

lazy_static! {
    static ref MOCK_TIMESTAMP: RwLock<u64> = RwLock::new(0);
}
/// This is a mock impl of TimestampGenerator which can be used during test to replace the global
/// TimestampGenerator in order to mock system time.
#[derive(Debug)]
pub struct MockTimestampGenerator {}

/// We don't want to wrap the global TimestampGenerator in a Mutex or RwLock, we prefer to simply
/// have a global object which can be
impl MockTimestampGenerator {
    pub fn new() -> MockTimestampGenerator {
        let mut mock_timestamp_global = MOCK_TIMESTAMP.write().unwrap();
        *mock_timestamp_global = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        MockTimestampGenerator {}
    }
}
impl AbstractTimestampGenerator for MockTimestampGenerator {
    fn get_timestamp(&self) -> u64 {
        let timestamp = MOCK_TIMESTAMP.read().unwrap();
        *timestamp
    }
    fn advance(&self, time_difference: u64) {
        let mut mock_timestamp_global = MOCK_TIMESTAMP.write().unwrap();
        *mock_timestamp_global = *mock_timestamp_global + time_difference;
    }
}
