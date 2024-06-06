use std::io::Cursor;

use fedimint_core::core::OperationId;
use fedimint_core::db::DatabaseTransaction;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::impl_db_record;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::util::BoxFuture;
use serde::Serialize;
use strum_macros::EnumIter;

#[derive(Clone, EnumIter, Debug)]
pub enum DbKeyPrefix {
    NextPegInTweakIndex = 0x2c,
}

impl std::fmt::Display for DbKeyPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, Encodable, Decodable, Serialize)]
pub struct NextPegInTweakIndexKey;

impl_db_record!(
    key = NextPegInTweakIndexKey,
    value = u64,
    db_prefix = DbKeyPrefix::NextPegInTweakIndex,
);

/// Migrates `SubmittedOfferV0` to `SubmittedOffer` and `ConfirmedInvoiceV0` to
/// `ConfirmedInvoice`
pub(crate) fn get_v0_migrated_state(
    operation_id: OperationId,
    cursor: &mut Cursor<&[u8]>,
    _dbtx: &mut DatabaseTransaction<'_>,
) -> anyhow::Result<Option<(Vec<u8>, OperationId)>> {
    fedimint_core::util::write_log_sync(&format!("inside get_v0_migrated_state"))?;
    // fedimint_core::runtime::block_on(fedimint_core::util::write_log(&format!(
    // "inside get_v1_migrated_state" )))?;

    // fedimint_core::util::write_log(&format!("inside
    // get_v1_migrated_state")).await?;
    let decoders = ModuleDecoderRegistry::default();
    let wallet_sm_variant = u16::consensus_decode(cursor, &decoders)?;

    fedimint_core::util::write_log_sync(&format!("wallet_sm_variant: {wallet_sm_variant:?}"))?;

    let wallet_sm_len = u16::consensus_decode(cursor, &decoders)?;
    let operation_id = OperationId::consensus_decode(cursor, &decoders)?;

    fedimint_core::util::write_log_sync(&format!("wallet_sm_len: {wallet_sm_len:?}"))?;
    fedimint_core::util::write_log_sync(&format!("operation_id: {operation_id:?}"))?;

    // think I found the issue! need to consider Deposit/Withdraw, was skipping
    // assuming Deposit
    match wallet_sm_variant {
        0 => {
            fedimint_core::util::write_log_sync(&format!("matched 0 wallet_sm_variant, Deposit"))?;

            let deposit_sm_variant = u16::consensus_decode(cursor, &decoders)?;

            match deposit_sm_variant {
                0 => {
                    let created_sm_len = u16::consensus_decode(cursor, &decoders)?;
                    fedimint_core::util::write_log_sync(&format!(
                        "created_sm_len: {created_sm_len:?}"
                    ))?;

                    let created_deposit_state =
                        crate::deposit::CreatedDepositState::consensus_decode(cursor, &decoders)?;
                    fedimint_core::util::write_log_sync(&format!(
                        "created_deposit_state: {created_deposit_state:?}"
                    ))?;
                }
                1 => {
                    fedimint_core::util::write_log_sync(&format!(
                        "matched 1 wallet_sm_variant, WaitingForConfirmations"
                    ))?;
                    let waiting_for_confirmations_sm_len =
                        u16::consensus_decode(cursor, &decoders)?;
                    fedimint_core::util::write_log_sync(&format!(
                        "waiting_for_confirmations_sm_len: {waiting_for_confirmations_sm_len:?}"
                    ))?;

                    let waiting_for_confirmations_deposit_state =
                        crate::deposit::WaitingForConfirmationsDepositState::consensus_decode(
                            cursor, &decoders,
                        )?;
                    fedimint_core::util::write_log_sync(&format!(
                        "waiting_for_confirmations_deposit_state: {waiting_for_confirmations_deposit_state:?}"
                    ))?;
                }
                2 => {
                    fedimint_core::util::write_log_sync(&format!(
                        "matched 2 wallet_sm_variant, Claiming"
                    ))?;
                    let created_sm_len = u16::consensus_decode(cursor, &decoders)?;
                    fedimint_core::util::write_log_sync(&format!(
                        "created_sm_len: {created_sm_len:?}"
                    ))?;

                    #[derive(Debug, Clone, Eq, PartialEq, Hash, Decodable, Encodable)]
                    pub struct ClaimingDepositStateV0 {
                        /// Fedimint transaction id in which the deposit is
                        /// being claimed.
                        pub(crate) transaction_id: fedimint_core::TransactionId,
                        pub(crate) change: Vec<fedimint_core::OutPoint>,
                    }

                    let claiming_deposit_state =
                        ClaimingDepositStateV0::consensus_decode(cursor, &decoders)?;
                    fedimint_core::util::write_log_sync(&format!(
                        "claiming_deposit_state: {claiming_deposit_state:?}"
                    ))?;

                    // TODO: query dbtx to get btc tx details using operation_id
                }
                other => panic!("unknown variant: {other:?}"),
            }
        }
        1 => {
            fedimint_core::util::write_log_sync(&format!("matched 1 wallet_sm_variant, Withdraw"))?;

            let withdraw_sm_variant = u16::consensus_decode(cursor, &decoders)?;
            fedimint_core::util::write_log_sync(&format!(
                "withdraw_sm_variant: {withdraw_sm_variant:?}"
            ))?;
        }
        other => {
            panic!("recived other wallet state variant: {other}");
        }
    }

    Ok(None)
    // #[derive(Debug, Clone, Decodable)]
    // pub struct LightningReceiveConfirmedInvoiceV0 {
    //     invoice: Bolt11Invoice,
    //     receiving_key: KeyPair,
    // }

    // let decoders = ModuleDecoderRegistry::default();
    // let ln_sm_variant = u16::consensus_decode(cursor, &decoders)?;

    // // If the state machine is not a receive state machine, return None
    // if ln_sm_variant != 2 {
    //     return Ok(None);
    // }

    // let _ln_sm_len = u16::consensus_decode(cursor, &decoders)?;
    // let _operation_id = OperationId::consensus_decode(cursor, &decoders)?;
    // let receive_sm_variant = u16::consensus_decode(cursor, &decoders)?;

    // let new = match receive_sm_variant {
    //     // SubmittedOfferV0
    //     0 => {
    //         let _receive_sm_len = u16::consensus_decode(cursor, &decoders)?;

    //         let v0 =
    // LightningReceiveSubmittedOfferV0::consensus_decode(cursor, &decoders)?;

    //         let new_offer = LightningReceiveSubmittedOffer {
    //             offer_txid: v0.offer_txid,
    //             invoice: v0.invoice,
    //             receiving_key: ReceivingKey::Personal(v0.payment_keypair),
    //         };
    //         let new_recv = LightningReceiveStateMachine {
    //             operation_id,
    //             state: LightningReceiveStates::SubmittedOffer(new_offer),
    //         };
    //         LightningClientStateMachines::Receive(new_recv)
    //     }
    //     // ConfirmedInvoiceV0
    //     2 => {
    //         let _receive_sm_len = u16::consensus_decode(cursor, &decoders)?;
    //         let confirmed_old =
    //             LightningReceiveConfirmedInvoiceV0::consensus_decode(cursor,
    // &decoders)?;         let confirmed_new =
    // LightningReceiveConfirmedInvoice {             invoice:
    // confirmed_old.invoice,             receiving_key:
    // ReceivingKey::Personal(confirmed_old.receiving_key),         };
    //         LightningClientStateMachines::Receive(LightningReceiveStateMachine {
    //             operation_id,
    //             state:
    // LightningReceiveStates::ConfirmedInvoice(confirmed_new),         })
    //     }
    //     _ => return Ok(None),
    // };

    // let bytes = new.consensus_encode_to_vec();
    // Ok(Some((bytes, operation_id)))
}

// #[cfg(test)]
// mod tests {
//     todo!()
// }
