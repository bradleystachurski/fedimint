use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use fedimint_core::Amount;
use fedimint_core::core::{ModuleKind, OperationId};
use fedimint_eventlog::{Event, EventKind, EventPersistence};
use serde::{Deserialize, Serialize};

use crate::client_db::TweakIdx;

/// Event that is emitted when the client pegs-out ecash onchain
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WithdrawRequest {
    /// The bitcoin transaction ID
    pub txid: Txid,
}

impl Event for WithdrawRequest {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);

    const KIND: EventKind = EventKind::from_static("withdraw-request");
    const PERSISTENCE: EventPersistence = EventPersistence::Persistent;
}

/// Event that is emitted when the client confirms an onchain deposit.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DepositConfirmed {
    /// The bitcoin transaction ID
    pub txid: Txid,

    /// The out index of the deposit transaction
    pub out_idx: u32,

    /// The amount being deposited
    pub amount: Amount,
}

impl Event for DepositConfirmed {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("deposit-confirmed");
    const PERSISTENCE: EventPersistence = EventPersistence::Persistent;
}

/// Event emitted when a peg-out (send to onchain) operation is initiated.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SendPaymentEvent {
    pub operation_id: OperationId,
    pub amount: bitcoin::Amount,
    pub fee: bitcoin::Amount,
}

impl Event for SendPaymentEvent {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("payment-send");
    const PERSISTENCE: EventPersistence = EventPersistence::Persistent;
}

/// Status of a send (peg-out) operation.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum SendPaymentStatus {
    /// The peg-out was successful, includes the bitcoin transaction ID.
    Success(Txid),
    /// The peg-out was aborted.
    Aborted,
}

/// Event emitted when a send (peg-out) operation reaches a final state.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SendPaymentStatusEvent {
    pub operation_id: OperationId,
    pub status: SendPaymentStatus,
}

impl Event for SendPaymentStatusEvent {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("payment-send-status");
    const PERSISTENCE: EventPersistence = EventPersistence::Persistent;
}

// Emitted when a deposit is confirmed
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ReceivePaymentEvent {
    pub operation_id: OperationId,
    pub amount: Amount,
    pub txid: Txid,
    pub address: Address<NetworkUnchecked>,
    pub out_idx: u32,
}

impl Event for ReceivePaymentEvent {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("payment-receive");
    const PERSISTENCE: EventPersistence = EventPersistence::Persistent;
}

/// Transient event emitted when a deposit is detected in the mempool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositInMempool {
    pub tweak_idx: TweakIdx,
    pub address: Address<NetworkUnchecked>,
    pub txid: Txid,
    pub out_idx: u32,
    pub amount: Amount,
}

impl Event for DepositInMempool {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("deposit-in-mempool");
    const PERSISTENCE: EventPersistence = EventPersistence::Transient;
}

/// Transient event emitted when a deposit is awaiting confirmations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAwaitingConfs {
    pub tweak_idx: TweakIdx,
    pub address: Address<NetworkUnchecked>,
    pub txid: Txid,
    pub out_idx: u32,
    pub amount: Amount,
    pub confirmations: u64,
    pub required: u64,
    pub tx_block_height: u64,
    pub consensus_block_height: u64,
}

impl Event for DepositAwaitingConfs {
    const MODULE: Option<ModuleKind> = Some(fedimint_wallet_common::KIND);
    const KIND: EventKind = EventKind::from_static("deposit-awaiting-confs");
    const PERSISTENCE: EventPersistence = EventPersistence::Transient;
}
