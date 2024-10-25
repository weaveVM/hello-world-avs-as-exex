use crate::utils::constants::HELLO_WORLD_CONTRACT_ADDRESS;
use dotenv::dotenv;
use ethers::prelude::*;
use ethers::signers::LocalWallet;
use ethers::utils::keccak256;
use eyre::Result;
use once_cell::sync::Lazy;
use std::{env, str::FromStr, sync::Arc};

// Environment variables
static KEY: Lazy<String> =
    Lazy::new(|| env::var("HOLESKY_PRIVATE_KEY").expect("Private key not set"));
pub static RPC_URL: Lazy<String> =
    Lazy::new(|| env::var("HOLESKY_RPC_URL").expect("RPC URL not set"));

// Generate contract bindings
abigen!(
    HelloWorldServiceManager,
    "./json_abi/HelloWorldServiceManager.json"
);

pub async fn get_provider_and_avs_manager() -> Result<(Provider<Http>, Address)> {
    let provider = Provider::<Http>::try_from(RPC_URL.clone())?;
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
    let wallet = LocalWallet::from_str(&KEY)?;
    let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
    let contract = HelloWorldServiceManager::new(contract_address, client);

    // Create the message to sign
    let message = format!("Hello, {}", name);
    let msg_hash = keccak256(message.as_bytes());

    // Sign the message
    let signature = wallet.sign_message(&msg_hash).await?;

    println!("Signing and responding to task {}", task_index);

    // Create the Task struct using the generated type
    let task = hello_world_service_manager::Task {
        name: name.clone(),
        task_created_block,
    };

    // Send the transaction
    contract
        .respond_to_task(task, task_index, signature.to_vec().into())
        .send()
        .await?
        .await?;

    println!("Successfully responded to task {}", task_index);
    Ok(())
}

pub async fn monitor_new_tasks_of_block(
    provider: Provider<Http>,
    contract_address: H160,
    block_number: u32,
) -> Result<()> {
    dotenv().ok();
    let client = Arc::new(SignerMiddleware::new(
        provider.clone(),
        LocalWallet::from_str(&KEY)?,
    ));
    let contract = HelloWorldServiceManager::new(contract_address, client.clone());

    println!("Starting task monitoring from block {}", block_number);

    let filter = contract.new_task_created_filter().from_block(block_number);

    loop {
        let events = contract
            .new_task_created_filter()
            .from_block(block_number)
            .query()
            .await?;

        for event in events {
            println!(
                "New task detected: {} at index {}",
                event.task.name, event.task_index
            );

            sign_and_respond_to_task(
                provider.clone(),
                contract_address,
                event.task_index,
                event.task.name,
                event.task.task_created_block,
            )
            .await?;
        }
    }
}
