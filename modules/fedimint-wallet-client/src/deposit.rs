use std::sync::Arc;
use std::time::{Duration, SystemTime};

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
use secp256k1::KeyPair;
use tracing::{debug, instrument, trace, warn};

use crate::api::WalletFederationApi;
use crate::{WalletClientContext, WalletClientStates};

const TRANSACTION_STATUS_FETCH_INTERVAL: Duration = Duration::from_secs(1);

// FIXME: deal with RBF
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

#[instrument(skip_all, level = "debug")]
async fn await_btc_transaction_confirmed(
    context: WalletClientContext,
    global_context: DynGlobalClientContext,
    waiting_state: WaitingForConfirmationsDepositState,
) -> (TxOutProof, bitcoin::Transaction, u32) {
    loop {
        fedimint_core::util::write_log(&format!("starting loop of await confirmed"))
            .await
            .unwrap();
        // TODO: make everything subscriptions
        // Wait for confirmation
        let consensus_block_count = match global_context
            .module_api()
            .fetch_consensus_block_count()
            .await
        {
            Ok(consensus_block_count) => consensus_block_count,
            Err(e) => {
                warn!("Failed to fetch consensus block count from federation: {e}");
                sleep(TRANSACTION_STATUS_FETCH_INTERVAL).await;
                continue;
            }
        };
        debug!(consensus_block_count, "Fetched consensus block count");

        let (confirmation_block_count, confirmed_tx, confirmed_out_idx) = match context
            .rpc
            .get_tx_block_height(&waiting_state.btc_transaction.txid())
            .await
        {
            Ok(Some(confirmation_height)) => (
                Some(confirmation_height + 1),
                waiting_state.btc_transaction.clone(),
                waiting_state.out_idx,
            ),
            Ok(None) => {
                fedimint_core::util::write_log(&format!(
                    "inside match arm for no confirmation height for ancestor"
                ))
                .await
                .unwrap();

                let script = context
                    .wallet_descriptor
                    .tweak(&waiting_state.tweak_key.public_key(), &context.secp)
                    .script_pubkey();

                let script_history = context.rpc.get_script_history(&script).await.unwrap();
                let filtered_script_history = script_history
                    .iter()
                    .filter(|tx| tx.txid() != waiting_state.btc_transaction.txid());
                fedimint_core::util::write_log(&format!(
                    "filtered_script_history: {filtered_script_history:?}"
                ))
                .await
                .unwrap();
                // perhaps I can make this generic for bitcoind and esplora by iterating over
                // all instead of just maybe
                let maybe_rbf_tx = script_history
                    .iter()
                    .filter(|tx| tx.txid() != waiting_state.btc_transaction.txid())
                    // .find(|tx| {
                    //     tx.input.iter().any(|input| {
                    //         waiting_state
                    //             .btc_transaction
                    //             .input
                    //             .iter()
                    //             .any(|ancestor_input| ancestor_input == input)
                    //     })
                    // });
                    // this works, need to compare specifically outpoints not full input
                    .find(|tx| {
                        tx.input.iter().any(|input| {
                            waiting_state
                                .btc_transaction
                                .input
                                .iter()
                                .any(|ancestor_input| {
                                    ancestor_input.previous_output == input.previous_output
                                })
                        })
                    });

                fedimint_core::util::write_log(&format!("maybe_rbf_tx: {maybe_rbf_tx:?}"))
                    .await
                    .unwrap();

                let ancestor_output = waiting_state
                    .btc_transaction
                    .output
                    .iter()
                    .nth(waiting_state.out_idx as usize)
                    .expect(
                        "Bad state: deposit transaction must contain output for out_idx: {out_idx}",
                    );

                fedimint_core::util::write_log(&format!("ancestor_output: {ancestor_output:?}"))
                    .await
                    .unwrap();
                match maybe_rbf_tx {
                    None => (
                        None,
                        waiting_state.btc_transaction.clone(),
                        waiting_state.out_idx,
                    ),
                    Some(rbf_tx) => {
                        let out_idx = rbf_tx
                            .output
                            .iter()
                            .position(|output| {
                                ancestor_output.script_pubkey == *output.script_pubkey
                            })
                            .expect("must exist since script_history was retrieved for this script")
                            as u32;
                        fedimint_core::util::write_log(&format!("out_idx: {out_idx:?}"))
                            .await
                            .unwrap();
                        fedimint_core::util::write_log(&format!("rbf_txid: {:?}", rbf_tx.txid()))
                            .await
                            .unwrap();

                        match context.rpc.get_tx_block_height(&rbf_tx.txid()).await {
                            Ok(Some(confirmation_height)) => {
                                fedimint_core::util::write_log(&format!(
                                    "found confirmation height for rbf tx: {confirmation_height:?}"
                                ))
                                .await
                                .unwrap();
                                (Some(confirmation_height + 1), rbf_tx.clone(), out_idx)
                            }
                            Ok(None) => (None, rbf_tx.clone(), out_idx),
                            Err(e) => {
                                warn!("Failed to fetch confirmation height: {e:?}");
                                (None, rbf_tx.clone(), out_idx)
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch confirmation height: {e:?}");
                (
                    None,
                    waiting_state.btc_transaction.clone(),
                    waiting_state.out_idx,
                )
            }
        };

        debug!(
            ?confirmation_block_count,
            "Fetched confirmation block count"
        );

        if !confirmation_block_count
            .map(|confirmation_block_count| consensus_block_count >= confirmation_block_count)
            .unwrap_or(false)
        {
            trace!("Not confirmed yet, confirmation_block_count={confirmation_block_count:?}, consensus_block_count={consensus_block_count}");
            sleep(TRANSACTION_STATUS_FETCH_INTERVAL).await;
            continue;
        }

        // Get txout proof
        let txout_proof = match context.rpc.get_txout_proof(confirmed_tx.txid()).await {
            Ok(txout_proof) => txout_proof,
            Err(e) => {
                warn!("Failed to fetch transaction proof: {e:?}");
                sleep(TRANSACTION_STATUS_FETCH_INTERVAL).await;
                continue;
            }
        };

        debug!(proof_block_hash = ?txout_proof.block_header.block_hash(), "Generated merkle proof");

        return (txout_proof, confirmed_tx, confirmed_out_idx);
    }
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
        confirmed_tx,
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
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
pub struct TimedOutDepositState {}
