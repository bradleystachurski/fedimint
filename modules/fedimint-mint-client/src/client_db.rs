use std::io::Cursor;

use fedimint_client::module::init::recovery::RecoveryFromHistoryCommon;
use fedimint_client::module::{IdxRange, OutPointRange};
use fedimint_core::core::OperationId;
use fedimint_core::db::{DatabaseTransaction, IDatabaseTransactionOpsCoreTyped as _};
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::{impl_db_lookup, impl_db_record, Amount};
use fedimint_mint_common::Nonce;
use serde::Serialize;
use strum_macros::EnumIter;

use crate::backup::recovery::MintRecoveryState;
use crate::input::{MintInputCommon, MintInputStateMachine, MintInputStateMachineV0};
use crate::oob::{MintOOBStateMachine, MintOOBStateMachineV0, MintOOBStates, MintOOBStatesV0};
use crate::output::{MintOutputCommon, MintOutputStateMachine, MintOutputStateMachineV0};
use crate::{MintClientStateMachines, SpendableNoteUndecoded};

#[repr(u8)]
#[derive(Clone, EnumIter, Debug)]
pub enum DbKeyPrefix {
    Note = 0x20,
    NextECashNoteIndex = 0x2a,
    CancelledOOBSpend = 0x2b,
    RecoveryState = 0x2c,
    RecoveryFinalized = 0x2d,
}

impl std::fmt::Display for DbKeyPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct NoteKey {
    pub amount: Amount,
    pub nonce: Nonce,
}

#[derive(Debug, Clone, Encodable, Decodable)]
pub struct NoteKeyPrefix;

impl_db_record!(
    key = NoteKey,
    value = SpendableNoteUndecoded,
    db_prefix = DbKeyPrefix::Note,
);
impl_db_lookup!(key = NoteKey, query_prefix = NoteKeyPrefix);

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct NextECashNoteIndexKey(pub Amount);

#[derive(Debug, Clone, Encodable, Decodable)]
pub struct NextECashNoteIndexKeyPrefix;

impl_db_record!(
    key = NextECashNoteIndexKey,
    value = u64,
    db_prefix = DbKeyPrefix::NextECashNoteIndex,
);
impl_db_lookup!(
    key = NextECashNoteIndexKey,
    query_prefix = NextECashNoteIndexKeyPrefix
);

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct RecoveryStateKey;

#[derive(Debug, Clone, Encodable, Decodable)]
pub struct RestoreStateKeyPrefix;

impl_db_record!(
    key = RecoveryStateKey,
    value = (MintRecoveryState, RecoveryFromHistoryCommon),
    db_prefix = DbKeyPrefix::RecoveryState,
);

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct RecoveryFinalizedKey;

#[derive(Debug, Clone, Encodable, Decodable)]
pub struct RecoveryFinalizedKeyPrefix;

impl_db_record!(
    key = RecoveryFinalizedKey,
    value = bool,
    db_prefix = DbKeyPrefix::RecoveryFinalized,
);

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct CancelledOOBSpendKey(pub OperationId);

#[derive(Debug, Clone, Encodable, Decodable, Serialize)]
pub struct CancelledOOBSpendKeyPrefix;

impl_db_record!(
    key = CancelledOOBSpendKey,
    value = (),
    db_prefix = DbKeyPrefix::CancelledOOBSpend,
    notify_on_modify = true,
);

impl_db_lookup!(
    key = CancelledOOBSpendKey,
    query_prefix = CancelledOOBSpendKeyPrefix,
);

pub async fn migrate_to_v1(
    dbtx: &mut DatabaseTransaction<'_>,
) -> anyhow::Result<Option<(Vec<(Vec<u8>, OperationId)>, Vec<(Vec<u8>, OperationId)>)>> {
    // between v0 and v1, we changed the format of `MintRecoveryState`, and instead
    // of migrating it, we can just delete it, so the recovery will just start
    // again, ignoring any existing state from before the migration
    dbtx.remove_entry(&RecoveryStateKey).await;

    Ok(None)
}

/// Migrates `MintClientStateMachinesV0`
pub(crate) fn migrate_state_to_v2(
    operation_id: OperationId,
    cursor: &mut Cursor<&[u8]>,
) -> anyhow::Result<Option<(Vec<u8>, OperationId)>> {
    let decoders = ModuleDecoderRegistry::default();

    let mint_client_state_machine_variant = u16::consensus_decode(cursor, &decoders)?;

    let new_mint_state_machine = match mint_client_state_machine_variant {
        0 => {
            let _output_sm_len = u16::consensus_decode(cursor, &decoders)?;
            let old_state = MintOutputStateMachineV0::consensus_decode(cursor, &decoders)?;

            MintClientStateMachines::Output(MintOutputStateMachine {
                common: MintOutputCommon {
                    operation_id: old_state.common.operation_id,
                    out_point_range: OutPointRange::new_single(
                        old_state.common.out_point.txid,
                        old_state.common.out_point.out_idx,
                    ),
                },
                state: old_state.state,
            })
        }
        1 => {
            let _input_sm_len = u16::consensus_decode(cursor, &decoders)?;
            let old_state = MintInputStateMachineV0::consensus_decode(cursor, &decoders)?;

            MintClientStateMachines::Input(MintInputStateMachine {
                common: MintInputCommon {
                    operation_id: old_state.common.operation_id,
                    out_point_range: OutPointRange::new(
                        old_state.common.txid,
                        IdxRange::new_single(old_state.common.input_idx),
                    ),
                },
                state: old_state.state,
            })
        }
        2 => {
            let _oob_sm_len = u16::consensus_decode(cursor, &decoders)?;
            let old_state = MintOOBStateMachineV0::consensus_decode(cursor, &decoders)?;

            let new_state = match old_state.state {
                MintOOBStatesV0::Created(created) => MintOOBStates::Created(created),
                MintOOBStatesV0::UserRefund(refund) => MintOOBStates::UserRefund(refund),
                MintOOBStatesV0::TimeoutRefund(refund) => MintOOBStates::TimeoutRefund(refund),
            };
            MintClientStateMachines::OOB(MintOOBStateMachine {
                operation_id: old_state.operation_id,
                state: new_state,
            })
        }
        _ => return Ok(None),
    };
    Ok(Some((
        new_mint_state_machine.consensus_encode_to_vec(),
        operation_id,
    )))
}
