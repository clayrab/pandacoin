use std::{sync::RwLock, time::{SystemTime, UNIX_EPOCH}};

use crate::timestamp_generator::TimestampGenerator;

/// This is a mock impl of TimestampGenerator which can be used during test to replace the global
/// TimestampGenerator in order to mock system time.
pub struct MockTimestampGenerator {
}

/// We don't want to wrap the global TimestampGenerator in a Mutex or RwLock, we prefer to simply
/// have a global object which can be
impl MockTimestampGenerator {
    pub fn new() -> MockTimestampGenerator {
        println!("new MockTimestampGenerator");
        let mut mock_timestamp_global = MOCK_TIMESTAMP_GLOBAL.write().unwrap();
        *mock_timestamp_global = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        MockTimestampGenerator {
        }
    }

}
impl TimestampGenerator for MockTimestampGenerator {
    fn get_timestamp(&self) -> u64 {
        let timestamp = MOCK_TIMESTAMP_GLOBAL.read().unwrap();
        *timestamp
    }
    fn advance(&self, time_difference: u64) {
        println!("advance MockTimestampGenerator");
        let mut mock_timestamp_global = MOCK_TIMESTAMP_GLOBAL.write().unwrap();
        *mock_timestamp_global = *mock_timestamp_global + time_difference;
    }
}