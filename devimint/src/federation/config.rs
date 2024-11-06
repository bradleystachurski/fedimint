use bitcoincore_rpc::bitcoin::Network;
use fedimint_core::bitcoin_migration::bitcoin30_to_bitcoin32_network;
use fedimint_core::config::{EmptyGenParams, ServerModuleConfigGenParamsRegistry};
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::envs::{is_env_var_set, BitcoinRpcConfig, FM_USE_UNKNOWN_MODULE_ENV};
use fedimint_core::fee_consensus::FeeConsensus;
use fedimint_core::module::ServerModuleInit as _;
use fedimint_ln_server::common::config::{
    LightningGenParams, LightningGenParamsConsensus, LightningGenParamsLocal,
};
use fedimint_ln_server::LightningInit;
use fedimint_meta_server::{MetaGenParams, MetaInit};
use fedimint_mint_server::common::config::{MintGenParams, MintGenParamsConsensus};
use fedimint_mint_server::MintInit;
use fedimint_unknown_server::common::config::UnknownGenParams;
use fedimint_unknown_server::UnknownInit;
use fedimint_wallet_client::config::{
    WalletGenParams, WalletGenParamsConsensus, WalletGenParamsLocal,
};
use fedimint_wallet_server::WalletInit;
use fedimintd::default_esplora_server;
use fedimintd::envs::FM_DISABLE_META_MODULE_ENV;
use serde::{Deserialize, Serialize};

use crate::version_constants::{VERSION_0_4_0_ALPHA, VERSION_0_5_0_ALPHA};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OldMintGenParams {
    pub local: EmptyGenParams,
    pub consensus: OldMintGenParamsConsensus,
}

// this is gross, delete later after whatever
// necessary to support old FeeConsensus fields
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Encodable, Decodable)]
pub struct OldFeeConsensus {
    pub note_issuance_abs: fedimint_core::Amount,
    pub note_spend_abs: fedimint_core::Amount,
}

impl Default for OldFeeConsensus {
    fn default() -> Self {
        Self {
            note_issuance_abs: fedimint_core::Amount::ZERO,
            note_spend_abs: fedimint_core::Amount::ZERO,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OldMintGenParamsConsensus {
    denomination_base: u16,
    fee_consensus: OldFeeConsensus,
}

impl OldMintGenParamsConsensus {
    pub fn new(denomination_base: u16, fee_consensus: OldFeeConsensus) -> Self {
        Self {
            denomination_base,
            fee_consensus,
        }
    }
}

impl fedimint_core::config::ModuleInitParams for OldMintGenParams {
    type Local = EmptyGenParams;
    type Consensus = OldMintGenParamsConsensus;

    fn from_parts(local: Self::Local, consensus: Self::Consensus) -> Self {
        Self { local, consensus }
    }

    fn to_parts(self) -> (Self::Local, Self::Consensus) {
        (self.local, self.consensus)
    }
}

/// Duplicate default fedimint module setup
pub fn attach_default_module_init_params(
    bitcoin_rpc: &BitcoinRpcConfig,
    module_init_params: &mut ServerModuleConfigGenParamsRegistry,
    network: Network,
    finality_delay: u32,
    fedimintd_version: &semver::Version,
) {
    module_init_params.attach_config_gen_params(
        LightningInit::kind(),
        LightningGenParams {
            local: LightningGenParamsLocal {
                bitcoin_rpc: bitcoin_rpc.clone(),
            },
            consensus: LightningGenParamsConsensus {
                network: bitcoin30_to_bitcoin32_network(&network),
            },
        },
    );

    if fedimintd_version >= &VERSION_0_5_0_ALPHA {
        module_init_params.attach_config_gen_params(
            MintInit::kind(),
            MintGenParams {
                local: EmptyGenParams::default(),
                consensus: MintGenParamsConsensus::new(2, FeeConsensus::zero()),
            },
        )
    } else {
        module_init_params.attach_config_gen_params(
            MintInit::kind(),
            OldMintGenParams {
                local: EmptyGenParams::default(),
                consensus: OldMintGenParamsConsensus::new(2, OldFeeConsensus::default()),
            },
        )
    };

    module_init_params.attach_config_gen_params(
        WalletInit::kind(),
        WalletGenParams {
            local: WalletGenParamsLocal {
                bitcoin_rpc: bitcoin_rpc.clone(),
            },
            consensus: WalletGenParamsConsensus {
                network: bitcoin30_to_bitcoin32_network(&network),
                finality_delay,
                client_default_bitcoin_rpc: default_esplora_server(network),
                fee_consensus: fedimint_wallet_client::config::FeeConsensus::default(),
            },
        },
    );

    // TODO(support:v0.3): v0.4 introduced lnv2 modules, so we need to skip
    // attaching the module for old fedimintd versions
    if fedimintd_version >= &VERSION_0_4_0_ALPHA {
        module_init_params.attach_config_gen_params(
            fedimint_lnv2_server::LightningInit::kind(),
            fedimint_lnv2_common::config::LightningGenParams {
                local: fedimint_lnv2_common::config::LightningGenParamsLocal {
                    bitcoin_rpc: bitcoin_rpc.clone(),
                },
                consensus: fedimint_lnv2_common::config::LightningGenParamsConsensus {
                    fee_consensus: fedimint_core::fee_consensus::FeeConsensus::new_lnv2(1000)
                        .expect("Relative fee is within range"),
                    network: bitcoin30_to_bitcoin32_network(&network),
                },
            },
        );
    }

    if !is_env_var_set(FM_DISABLE_META_MODULE_ENV) {
        module_init_params.attach_config_gen_params(MetaInit::kind(), MetaGenParams::default());
    }

    if is_env_var_set(FM_USE_UNKNOWN_MODULE_ENV) {
        module_init_params
            .attach_config_gen_params(UnknownInit::kind(), UnknownGenParams::default());
    }
}
