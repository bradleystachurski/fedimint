use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::Context;
use fedimint_client::sm::{ClientSMDatabaseTransaction, State, StateTransition};
use fedimint_client::transaction::ClientInput;
use fedimint_client::DynGlobalClientContext;
use fedimint_core::core::OperationId;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::task::sleep;
use fedimint_core::txoproof::TxOutProof;
use fedimint_core::util::{retry, FibonacciBackoff};
use fedimint_core::{Amount, OutPoint, TransactionId};
use fedimint_wallet_common::tweakable::Tweakable;
use fedimint_wallet_common::txoproof::PegInProof;
use fedimint_wallet_common::WalletInput;
use futures::StreamExt;
use secp256k1::KeyPair;
use tracing::{debug, instrument, warn};

use crate::api::WalletFederationApi;
use crate::{WalletClientContext, WalletClientStates};

// FIXME: deal with multiple deposits
#[cfg_attr(doc, aquamarine::aquamarine)]
/// The state machine driving forward a deposit (aka peg-in).
///
/// ```mermaid
/// graph LR
///     Created -- Transaction seen --> AwaitingConfirmations["Waiting for confirmations"]
///     AwaitingConfirmations -- Confirmations received --> Claiming
///     AwaitingConfirmations -- RBF seen --> AwaitingConfirmations
///     AwaitingConfirmations -- "Retransmit seen tx (planned)" --> AwaitingConfirmations
///     Created -- "No transactions seen for [time]" --> Timeout["Timed out"]
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct DepositStateMachine {
    pub(crate) operation_id: OperationId,
    pub(crate) state: DepositStates,
}

impl State for DepositStateMachine {
    type ModuleContext = WalletClientContext;

    fn transitions(
        &self,
        context: &Self::ModuleContext,
        global_context: &DynGlobalClientContext,
    ) -> Vec<StateTransition<Self>> {
        match &self.state {
            DepositStates::Created(created_state) => {
                vec![
                    StateTransition::new(
                        await_created_btc_transaction_submitted(
                            context.clone(),
                            created_state.tweak_key,
                        ),
                        |_db, (btc_tx, out_idx), old_state| {
                            Box::pin(transition_tx_seen(old_state, btc_tx, out_idx))
                        },
                    ),
                    StateTransition::new(
                        await_deposit_address_timeout(created_state.timeout_at),
                        |_db, (), old_state| Box::pin(transition_deposit_timeout(old_state)),
                    ),
                ]
            }
            DepositStates::WaitingForConfirmations(waiting_state) => {
                let global_context = global_context.clone();
                vec![StateTransition::new(
                    await_btc_transaction_confirmed(
                        context.clone(),
                        global_context.clone(),
                        waiting_state.clone(),
                    ),
                    move |dbtx, (txout_proof, confirmed_tx, confirmed_out_idx), old_state| {
                        Box::pin(transition_btc_tx_confirmed(
                            dbtx,
                            global_context.clone(),
                            old_state,
                            txout_proof,
                            confirmed_tx,
                            confirmed_out_idx,
                        ))
                    },
                )]
            }
            DepositStates::Claiming(_) => {
                vec![]
            }
            DepositStates::TimedOut(_) => {
                vec![]
            }
        }
    }

    fn operation_id(&self) -> OperationId {
        self.operation_id
    }
}

async fn await_created_btc_transaction_submitted(
    context: WalletClientContext,
    tweak: KeyPair,
) -> (bitcoin::Transaction, u32) {
    let script = context
        .wallet_descriptor
        .tweak(&tweak.public_key(), &context.secp)
        .script_pubkey();

    retry(
        "watch_script_history",
        FibonacciBackoff::default()
            .with_min_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(10 * 60))
            .with_max_times(usize::MAX),
        || async {
            context.rpc.watch_script_history(&script).await?;
            Ok(())
        },
    )
    .await
    .expect("never fails");

    let find_submitted_tx = || async {
        let script_history = context.rpc.get_script_history(&script).await?;

        if script_history.len() > 1 {
            warn!("More than one transaction was sent to deposit address, only considering the first one");
        }

        let transaction = script_history.into_iter().next().ok_or(anyhow::anyhow!(
            "No transactions received yet for script {script:?}"
        ))?;

        let out_idx = transaction
            .output
            .iter()
            .position(|output| output.script_pubkey == script)
            .expect("TODO: handle invalid tx returned by API") as u32;

        Ok((transaction, out_idx))
    };

    retry(
        "await_created_btc_transaction_submitted",
        FibonacciBackoff::default()
            .with_min_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(15 * 60))
            .with_max_times(usize::MAX),
        find_submitted_tx,
    )
    .await
    .expect("never fails")
}

async fn transition_tx_seen(
    old_state: DepositStateMachine,
    btc_transaction: bitcoin::Transaction,
    out_idx: u32,
) -> DepositStateMachine {
    let DepositStateMachine {
        operation_id,
        state: old_state,
    } = old_state;

    match old_state {
        DepositStates::Created(created_state) => DepositStateMachine {
            operation_id,
            state: DepositStates::WaitingForConfirmations(WaitingForConfirmationsDepositState {
                tweak_key: created_state.tweak_key,
                btc_transaction,
                out_idx,
            }),
        },
        state => panic!("Invalid previous state: {state:?}"),
    }
}

async fn await_deposit_address_timeout(timeout_at: SystemTime) {
    if let Ok(time_until_deadline) = timeout_at.duration_since(fedimint_core::time::now()) {
        sleep(time_until_deadline).await;
    }
}

async fn transition_deposit_timeout(old_state: DepositStateMachine) -> DepositStateMachine {
    assert!(
        matches!(old_state.state, DepositStates::Created(_)),
        "Invalid previous state"
    );

    DepositStateMachine {
        operation_id: old_state.operation_id,
        state: DepositStates::TimedOut(TimedOutDepositState {}),
    }
}

struct ConfirmedTxDetails {
    block_count: u64,
    tx: bitcoin::Transaction,
    out_idx: u32,
}

async fn find_confirmed_rbf_tx(
    context: &WalletClientContext,
    waiting_state: &WaitingForConfirmationsDepositState,
) -> anyhow::Result<Option<ConfirmedTxDetails>> {
    let script = context
        .wallet_descriptor
        .tweak(&waiting_state.tweak_key.public_key(), &context.secp)
        .script_pubkey();

    let ancestor_output = waiting_state
        .btc_transaction
        .output
        .iter()
        .nth(waiting_state.out_idx as usize)
        .expect("Bad state: deposit transaction must contain output for out_idx: {out_idx}");

    // TODO: paging to ensure we check script's full history
    let script_history = context.rpc.get_script_history(&script).await?;
    let rbf_transactions = script_history.iter().filter(|tx| {
        // rbf transactions have a different txid
        tx.txid() != waiting_state.btc_transaction.txid()
            // any input spent by another unconfirmed tx is an rbf tx
            && tx.input.iter().any(|input| {
                waiting_state
                    .btc_transaction
                    .input
                    .iter()
                    .any(|ancestor_input| ancestor_input.previous_output == input.previous_output)
            })
    });

    let maybe_rbf_tx = futures::stream::iter(rbf_transactions)
        .filter_map(|rbf_tx| async {
            let out_idx = rbf_tx
                .output
                .iter()
                .position(|output| ancestor_output.script_pubkey == *output.script_pubkey)
                .expect("must exist since script_history was retrieved for this script")
                as u32;

            match context.rpc.get_tx_block_height(&rbf_tx.txid()).await {
                Ok(Some(confirmation_height)) => Some(ConfirmedTxDetails {
                    block_count: confirmation_height + 1,
                    tx: rbf_tx.clone(),
                    out_idx,
                }),
                Ok(None) => None,
                Err(_) => None,
            }
        })
        .boxed()
        .next()
        .await;

    Ok(maybe_rbf_tx)
}

#[instrument(skip_all, level = "debug")]
async fn await_btc_transaction_confirmed(
    context: WalletClientContext,
    global_context: DynGlobalClientContext,
    waiting_state: WaitingForConfirmationsDepositState,
) -> (TxOutProof, bitcoin::Transaction, u32) {
    let find_confirmed_tx = || async {
        // TODO: make everything subscriptions
        // Wait for confirmation
        let consensus_block_count = global_context
            .module_api()
            .fetch_consensus_block_count()
            .await
            .context("Failed to fetch consensus block count from federation")?;

        debug!(consensus_block_count, "Fetched consensus block count");

        let ConfirmedTxDetails {
            block_count: confirmation_block_count,
            tx: confirmed_tx,
            out_idx: confirmed_out_idx,
        } = match context
            .rpc
            .get_tx_block_height(&waiting_state.btc_transaction.txid())
            .await
            .context(format!(
                "Failed to fetch tx block height for txid {:?}",
                waiting_state.btc_transaction.txid()
            ))?
            .map(|confirmation_height| ConfirmedTxDetails {
                block_count: confirmation_height + 1,
                tx: waiting_state.btc_transaction.clone(),
                out_idx: waiting_state.out_idx,
            }) {
            Some(original_tx_details) => original_tx_details,
            // If the original transaction doesn't have any confirmations, check for any confirmed
            // rbf transactions
            None => find_confirmed_rbf_tx(&context, &waiting_state)
                .await?
                // If there are no rbf transactions, we error, causing another iteration of the
                // retry fn
                .ok_or(anyhow::anyhow!("No confirmed transactions"))?,
        };

        debug!(
            ?confirmation_block_count,
            "Fetched confirmation block count"
        );

        anyhow::ensure!(
            consensus_block_count >= confirmation_block_count,
            "Not enough confirmations yet, confirmation_block_count={:?}, consensus_block_count={:?}",
            confirmation_block_count,
            consensus_block_count
        );

        // Get txout proof
        let txout_proof = context
            .rpc
            .get_txout_proof(confirmed_tx.txid())
            .await
            .context(format!(
                "Failed to fetch transaction proof for txid {:?}",
                confirmed_tx.txid()
            ))?;

        debug!(proof_block_hash = ?txout_proof.block_header.block_hash(), "Generated merkle proof");

        Ok((txout_proof, confirmed_tx, confirmed_out_idx))
    };

    retry(
        "await_btc_transaction_confirmed",
        FibonacciBackoff::default()
            .with_min_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(15 * 60))
            .with_max_times(usize::MAX),
        find_confirmed_tx,
    )
    .await
    .expect("never fails")
}

async fn transition_btc_tx_confirmed(
    dbtx: &mut ClientSMDatabaseTransaction<'_, '_>,
    global_context: DynGlobalClientContext,
    old_state: DepositStateMachine,
    txout_proof: TxOutProof,
    confirmed_tx: bitcoin::Transaction,
    confirmed_out_idx: u32,
) -> DepositStateMachine {
    let awaiting_confirmation_state = match old_state.state {
        DepositStates::WaitingForConfirmations(s) => s,
        _ => panic!("Invalid previous state"),
    };

    let pegin_proof = PegInProof::new(
        txout_proof,
        confirmed_tx.clone(),
        confirmed_out_idx,
        awaiting_confirmation_state.tweak_key.public_key(),
    )
    .expect("TODO: handle API returning faulty proofs");

    let amount = Amount::from_sats(pegin_proof.tx_output().value);

    let wallet_input = WalletInput::new_v0(pegin_proof);

    let client_input = ClientInput::<WalletInput, WalletClientStates> {
        input: wallet_input,
        keys: vec![awaiting_confirmation_state.tweak_key],
        amount,
        state_machines: Arc::new(|_, _| vec![]),
    };

    let (fm_txid, change) = global_context.claim_input(dbtx, client_input).await;

    DepositStateMachine {
        operation_id: old_state.operation_id,
        state: DepositStates::Claiming(ClaimingDepositState {
            transaction_id: fm_txid,
            change,
            btc_transaction: confirmed_tx,
            btc_out_idx: confirmed_out_idx,
        }),
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub enum DepositStates {
    Created(CreatedDepositState),
    WaitingForConfirmations(WaitingForConfirmationsDepositState),
    Claiming(ClaimingDepositState),
    TimedOut(TimedOutDepositState),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct CreatedDepositState {
    pub(crate) tweak_key: KeyPair,
    pub(crate) timeout_at: SystemTime,
}

// #[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
// pub struct PendingDepositTransaction {
//     /// The bitcoin transaction is saved as soon as we see it so the
// transaction     /// can be re-transmitted if it's evicted from the mempool.
//     pub(crate) btc_transaction: bitcoin::Transaction,
//     /// Index of the deposit output
//     pub(crate) out_idx: u32,
// }

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct WaitingForConfirmationsDepositState {
    /// Key pair of which the public was used to tweak the federation's wallet
    /// descriptor. The secret key is later used to sign the fedimint claim
    /// transaction.
    tweak_key: KeyPair,
    /// The bitcoin transaction is saved as soon as we see it so the transaction
    /// can be re-transmitted if it's evicted from the mempool.
    pub(crate) btc_transaction: bitcoin::Transaction,
    /// Index of the deposit output
    pub(crate) out_idx: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct ClaimingDepositState {
    /// Fedimint transaction id in which the deposit is being claimed.
    pub(crate) transaction_id: TransactionId,
    pub(crate) change: Vec<OutPoint>,
    pub(crate) btc_transaction: bitcoin::Transaction,
    pub(crate) btc_out_idx: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct TimedOutDepositState {}
