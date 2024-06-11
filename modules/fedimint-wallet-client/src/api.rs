use bitcoin::Address;
use fedimint_api_client::api::{FederationApiExt, FederationResult, IModuleFederationApi};
use fedimint_core::module::ApiRequestErased;
use fedimint_core::task::{MaybeSend, MaybeSync};
use fedimint_core::{apply, async_trait_maybe_send};
use fedimint_wallet_common::endpoint_constants::{
    AVAILABLE_UTXOS_ENDPOINT, BLOCK_COUNT_ENDPOINT, PEG_OUT_FEES_ENDPOINT, WALLET_SUMMARY_ENDPOINT,
};
use fedimint_wallet_common::{PegOutFees, UTXOSummary, WalletSummary};

#[apply(async_trait_maybe_send!)]
pub trait WalletFederationApi {
    async fn fetch_consensus_block_count(&self) -> FederationResult<u64>;

    async fn fetch_peg_out_fees(
        &self,
        address: &Address,
        amount: bitcoin::Amount,
    ) -> FederationResult<Option<PegOutFees>>;

    async fn fetch_available_utxos(&self) -> FederationResult<Vec<UTXOSummary>>;

    async fn fetch_wallet_summary(&self) -> FederationResult<WalletSummary>;
}

#[apply(async_trait_maybe_send!)]
impl<T: ?Sized> WalletFederationApi for T
where
    T: IModuleFederationApi + MaybeSend + MaybeSync + 'static,
{
    async fn fetch_consensus_block_count(&self) -> FederationResult<u64> {
        self.request_current_consensus(
            BLOCK_COUNT_ENDPOINT.to_string(),
            ApiRequestErased::default(),
        )
        .await
    }

    async fn fetch_peg_out_fees(
        &self,
        address: &Address,
        amount: bitcoin::Amount,
    ) -> FederationResult<Option<PegOutFees>> {
        self.request_current_consensus(
            PEG_OUT_FEES_ENDPOINT.to_string(),
            ApiRequestErased::new((address, amount.to_sat())),
        )
        .await
    }

    async fn fetch_available_utxos(&self) -> FederationResult<Vec<UTXOSummary>> {
        self.request_current_consensus(
            AVAILABLE_UTXOS_ENDPOINT.to_string(),
            ApiRequestErased::default(),
        )
        .await
    }

    async fn fetch_wallet_summary(&self) -> FederationResult<WalletSummary> {
        self.request_current_consensus(
            WALLET_SUMMARY_ENDPOINT.to_string(),
            ApiRequestErased::default(),
        )
        .await
    }
}
