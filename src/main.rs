use crate::utils::monitor_tasks::{get_provider_and_avs_manager, monitor_new_tasks_of_block};
use futures::{Future, TryStreamExt};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;

mod utils;

async fn exex_init<Node: FullNodeComponents>(
    ctx: ExExContext<Node>,
) -> eyre::Result<impl Future<Output = eyre::Result<()>>> {
    Ok(exex_operator(ctx))
}

async fn exex_operator<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
    let (provider, avs_manager) = get_provider_and_avs_manager().await.unwrap();

    while let Some(notification) = ctx.notifications.try_next().await? {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                info!(committed_chain = ?new.range(), "Received commit");
            }
            ExExNotification::ChainReorged { old, new } => {
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            // Get all block's receipts
            let last_block_transactions_receipts = committed_chain
                .blocks_and_receipts()
                // Flat map the (block, receipts) tuple to just receipts
                .flat_map(|(_block, receipts)| receipts.iter().cloned())
                .collect();

            // monitor the AVS Service Manager tasks
            monitor_new_tasks_of_block(
                provider.clone(),
                avs_manager,
                last_block_transactions_receipts,
            )
            .await
            .unwrap();
            ctx.events
                .send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))?;
        }
    }

    Ok(())
}

fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("exex-operator", exex_init)
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}

#[cfg(test)]
mod tests {
    use reth_execution_types::{Chain, ExecutionOutcome};
    use reth_exex_test_utils::{test_exex_context, PollOnce};
    use std::pin::pin;

    #[tokio::test]
    async fn test_exex() -> eyre::Result<()> {
        let (ctx, mut handle) = test_exex_context().await?;
        let head = ctx.head;
        handle
            .send_notification_chain_committed(Chain::from_block(
                handle.genesis.clone(),
                ExecutionOutcome::default(),
                None,
            ))
            .await?;

        // Initialize the Execution Extension
        let mut exex = pin!(super::exex_init(ctx).await?);
        handle.assert_events_empty();
        exex.poll_once().await?;
        handle.assert_event_finished_height((head.number, head.hash).into())?;

        Ok(())
    }
}
