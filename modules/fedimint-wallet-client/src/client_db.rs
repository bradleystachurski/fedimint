use std::io::Cursor;

use fedimint_core::core::OperationId;
use fedimint_core::db::DatabaseTransaction;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::impl_db_record;
use fedimint_core::module::registry::ModuleDecoderRegistry;
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
    fedimint_core::util::write_log_sync(&format!("inside get_v1_migrated_state"))?;
    // fedimint_core::runtime::block_on(fedimint_core::util::write_log(&format!(
    // "inside get_v1_migrated_state" )))?;

    // fedimint_core::util::write_log(&format!("inside
    // get_v1_migrated_state")).await?;
    let decoders = ModuleDecoderRegistry::default();
    let ln_sm_variant = u16::consensus_decode(cursor, &decoders)?;

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
