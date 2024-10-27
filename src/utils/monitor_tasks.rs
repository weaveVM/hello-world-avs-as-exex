use crate::utils::constants::{
    HELLO_WORLD_CONTRACT_ADDRESS, HOLESKY_RPC_URL, NEW_TASK_CREATED_EVENT_NAME,
};
use dotenv::dotenv;
use ethers::abi::ParamType;
use ethers::prelude::*;
use ethers::signers::LocalWallet;
use ethers::utils::keccak256;
use eyre::Result;
use once_cell::sync::Lazy;
use reth::primitives::{Log, Receipt, SealedBlockWithSenders};
use std::{env, str::FromStr, sync::Arc};
use zerocopy::IntoBytes;

static KEY: Lazy<String> =
    Lazy::new(|| env::var("HOLESKY_PRIVATE_KEY").expect("Private key not set"));

// Generate contract bindings
abigen!(
    HelloWorldServiceManager,
    "./json_abi/HelloWorldServiceManager.json"
);

pub async fn get_provider_and_avs_manager() -> Result<(Provider<Http>, Address)> {
    let provider = Provider::<Http>::try_from(HOLESKY_RPC_URL)?;
    let contract_address = Address::from_str(HELLO_WORLD_CONTRACT_ADDRESS)?;
    Ok((provider, contract_address))
}

async fn sign_and_respond_to_task(
    provider: Provider<Http>,
    contract_address: H160,
    task_index: u32,
    name: String,
    task_created_block: u32,
) -> Result<()> {
    dotenv().ok();
    let wallet = LocalWallet::from_str(&KEY)?;
    let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
    let contract = HelloWorldServiceManager::new(contract_address, client);
    let message = format!("Hello, {}", name);
    let msg_hash = keccak256(message.as_bytes());
    let signature = wallet.sign_message(&msg_hash).await?;

    println!("Signing and responding to task {}", task_index);

    let task = hello_world_service_manager::Task {
        name: name.clone(),
        task_created_block,
    };

    contract
        .respond_to_task(task, task_index, signature.to_vec().into())
        .send()
        .await?
        .await?;

    println!("Successfully responded to task {}", task_index);
    Ok(())
}

pub async fn decode_new_task_created_event(
    log: &Log,
) -> Result<(u32, hello_world_service_manager::Task)> {
    // Get task index from first topic (index 1 since index 0 is event signature)
    let task_index = u32::from_be_bytes(log.topics()[1].as_bytes()[28..32].try_into()?);

    // Directly use the Bytes from LogData
    let data_bytes: &[u8] = log.data.data.as_ref();

    // Decode non-indexed parameters (the Task struct)
    let decoded = ethers::abi::decode(
        &[ParamType::Tuple(vec![
            ParamType::String,   // name
            ParamType::Uint(32), // taskCreatedBlock
        ])],
        data_bytes,
    )?;

    if let ethers::abi::Token::Tuple(values) = &decoded[0] {
        let task = hello_world_service_manager::Task {
            name: values[0].clone().into_string().unwrap(),
            task_created_block: values[1].clone().into_uint().unwrap().as_u32(),
        };

        Ok((task_index, task))
    } else {
        Err(eyre::eyre!("Failed to decode task data"))
    }
}

pub async fn monitor_new_tasks_of_block(
    provider: Provider<Http>,
    contract_address: Address,
    current_block_transactions_receipts: Vec<Option<Receipt>>,
    current_sealed_block_with_senders: &SealedBlockWithSenders,
) -> Result<()> {
    // now you can use current_sealed_block_with_senders for EDA task scanning purposes
    let event_signature = H256::from_str(NEW_TASK_CREATED_EVENT_NAME)?;

    let mut filtered_logs: Vec<Log> = Vec::new();
    for receipt in current_block_transactions_receipts.into_iter().flatten() {
        for log in receipt.logs {
            if log.address == Address::from(contract_address).as_bytes()
                && !log.topics().is_empty()
                && log.topics()[0] == event_signature.as_bytes()
            {
                filtered_logs.push(log);
            }
        }
    }

    for log in &filtered_logs {
        match decode_new_task_created_event(log).await {
            Ok((task_index, task)) => {
                println!("New task detected at index {:?}", task_index);

                sign_and_respond_to_task(
                    provider.clone(),
                    contract_address,
                    task_index,
                    task.name,
                    task.task_created_block,
                )
                .await?;
            }
            Err(e) => println!("Failed to decode event: {}", e),
        }
    }

    Ok(())
}
