use crate::panda_protos::RawBlockProto;

/// 
pub struct BlockFee {
    _timestamps: Vec<u64>
}

impl BlockFee {
    pub fn roll_forward(&mut self, _block: &RawBlockProto) {

    }

    pub fn roll_back(&mut self, _block: &RawBlockProto) {
 
    }

    pub fn get_next_fee(&self) -> u64 {
        0
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::test_utilities::mock_timestamp_generator::MockTimestampGenerator;

//     lazy_static! {
//         pub static ref TIMESTAMP_GENERATOR_GLOBAL: Box<dyn TimestampGenerator + Send + Sync> = Box::new(MockTimestampGenerator::new());
//     }

//     #[tokio::test]
//     async fn mock_timestamp_generator_test() {
//         let first_timestamp = TIMESTAMP_GENERATOR_GLOBAL.get_timestamp();
//         TIMESTAMP_GENERATOR_GLOBAL.advance(1000);
//         assert_eq!(first_timestamp + 1000, TIMESTAMP_GENERATOR_GLOBAL.get_timestamp());        
//     }
// }
