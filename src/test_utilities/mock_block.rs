// use crate::block::RawBlock;
// use crate::block_fee::BlockFee;
// use crate::keypair::Keypair;
// use crate::panda_protos::TransactionProto;
// use crate::types::Sha256Hash;

// // let public_key: PublicKey = PublicKey::from_str(
// //     "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
// // )
// // .unwrap();
// // let timestamp = create_timestamp();
// // let block_core = BlockCore::new(
// //     block_id,
// //     timestamp,
// //     previous_block_hash,
// //     public_key,
// //     TREASURY,
// //     0,
// //     transactions,
// // );
// // Block::new(block_core)
// use crate::panda_protos::transaction_proto::TxType;
// use crate::{protos::{OutputId, Output}, types::{PandaAddress}};
// use crate::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;

// pub fn make_mock_block(
//     prev_timestamp: u64,
//     previous_burnfee: u64,
//     previous_previous_burnfee: u64,
//     previous_block_hash: Sha256Hash,
//     previous_block_id: u64,
//     from_slip: Option<OutputId>,
// ) -> RawBlock {
//     let step: u64 = 10000;
//     let keypair = Keypair::new();
//     let timestamp = prev_timestamp + step;
//     let mut txs = vec![];
//     if !from_slip.is_none() {
//         let to_slip = Output::new(keypair.public_key().clone(), 10);
//         let tx_core = TransactionProto::new(
//             TIMESTAMP_GENERATOR_GLOBAL.get_timestamp(),
//             vec![from_slip.unwrap().clone()],
//             vec![to_slip.clone()],
//             TxType::Normal,
//             vec![104, 101, 108, 108, 111],
//         );
//         txs.push(Transaction::sign(tx_core));
//     }

//     //Block::new_mock(previous_block_hash, vec![tx.clone()], previous_block_id + 1)
//     let block_core = RawBlock::new(
//         previous_block_id + 1,
//         timestamp,
//         previous_block_hash,
//         keypair.public_key().clone(),
//         0,
//         burnfee,
//         txs,
//     );
//     Block::new(block_core)
// }