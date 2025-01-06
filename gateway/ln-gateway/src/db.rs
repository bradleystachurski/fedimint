use std::collections::BTreeMap;

use bitcoin::hashes::{sha256, Hash};
use fedimint_api_client::api::net::Connector;
use fedimint_core::config::FederationId;
use fedimint_core::db::{
    CoreMigrationFn, Database, DatabaseTransaction, DatabaseVersion, IDatabaseTransactionOpsCore,
    IDatabaseTransactionOpsCoreTyped, MigrationContext,
};
use fedimint_core::encoding::btc::NetworkLegacyEncodingWrapper;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::invite_code::InviteCode;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::{impl_db_lookup, impl_db_record, push_db_pair_items, secp256k1, Amount};
use fedimint_ln_common::serde_routing_fees;
use fedimint_lnv2_common::contracts::{IncomingContract, PaymentImage};
use fedimint_lnv2_common::gateway_api::PaymentFee;
use futures::{FutureExt, StreamExt};
use lightning_invoice::RoutingFees;
use rand::rngs::OsRng;
use rand::Rng;
use secp256k1::{Keypair, Secp256k1};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub trait GatewayDbExt {
    fn get_client_database(&self, federation_id: &FederationId) -> Database;
}

impl GatewayDbExt for Database {
    fn get_client_database(&self, federation_id: &FederationId) -> Database {
        let mut prefix = vec![DbKeyPrefix::ClientDatabase as u8];
        prefix.append(&mut federation_id.consensus_encode_to_vec());
        self.with_prefix(prefix)
    }
}

pub trait GatewayDbtxNcExt {
    async fn save_federation_config(&mut self, config: &FederationConfig);
    async fn load_federation_configs_v0(&mut self) -> BTreeMap<FederationId, FederationConfigV0>;
    async fn load_federation_configs(&mut self) -> BTreeMap<FederationId, FederationConfig>;
    async fn load_federation_config(
        &mut self,
        federation_id: FederationId,
    ) -> Option<FederationConfig>;
    async fn remove_federation_config(&mut self, federation_id: FederationId);

    /// Returns the keypair that uniquely identifies the gateway.
    async fn load_gateway_keypair(&mut self) -> Option<Keypair>;

    /// Returns the keypair that uniquely identifies the gateway.
    ///
    /// # Panics
    /// Gateway keypair does not exist.
    async fn load_gateway_keypair_assert_exists(&mut self) -> Keypair;

    /// Returns the keypair that uniquely identifies the gateway, creating it if
    /// it does not exist. Remember to commit the transaction after calling this
    /// method.
    async fn load_or_create_gateway_keypair(&mut self) -> Keypair;

    async fn save_new_preimage_authentication(
        &mut self,
        payment_hash: sha256::Hash,
        preimage_auth: sha256::Hash,
    );

    async fn load_preimage_authentication(
        &mut self,
        payment_hash: sha256::Hash,
    ) -> Option<sha256::Hash>;

    /// Saves a registered incoming contract, returning the previous contract
    /// with the same payment hash if it existed.
    async fn save_registered_incoming_contract(
        &mut self,
        federation_id: FederationId,
        incoming_amount: Amount,
        contract: IncomingContract,
    ) -> Option<RegisteredIncomingContract>;

    async fn load_registered_incoming_contract(
        &mut self,
        payment_image: PaymentImage,
    ) -> Option<RegisteredIncomingContract>;

    /// Reads and serializes structures from the gateway's database for the
    /// purpose for serializing to JSON for inspection.
    async fn dump_database(
        &mut self,
        prefix_names: Vec<String>,
    ) -> BTreeMap<String, Box<dyn erased_serde::Serialize + Send>>;
}

impl<Cap: Send> GatewayDbtxNcExt for DatabaseTransaction<'_, Cap> {
    async fn save_federation_config(&mut self, config: &FederationConfig) {
        let id = config.invite_code.federation_id();
        self.insert_entry(&FederationIdKey { id }, config).await;
    }

    async fn load_federation_configs_v0(&mut self) -> BTreeMap<FederationId, FederationConfigV0> {
        self.find_by_prefix(&FederationIdKeyPrefixV0)
            .await
            .map(|(key, config): (FederationIdKeyV0, FederationConfigV0)| (key.id, config))
            .collect::<BTreeMap<FederationId, FederationConfigV0>>()
            .await
    }

    async fn load_federation_configs(&mut self) -> BTreeMap<FederationId, FederationConfig> {
        self.find_by_prefix(&FederationIdKeyPrefix)
            .await
            .map(|(key, config): (FederationIdKey, FederationConfig)| (key.id, config))
            .collect::<BTreeMap<FederationId, FederationConfig>>()
            .await
    }

    async fn load_federation_config(
        &mut self,
        federation_id: FederationId,
    ) -> Option<FederationConfig> {
        self.get_value(&FederationIdKey { id: federation_id }).await
    }

    async fn remove_federation_config(&mut self, federation_id: FederationId) {
        self.remove_entry(&FederationIdKey { id: federation_id })
            .await;
    }

    async fn load_gateway_keypair(&mut self) -> Option<Keypair> {
        self.get_value(&GatewayPublicKey).await
    }

    async fn load_gateway_keypair_assert_exists(&mut self) -> Keypair {
        self.get_value(&GatewayPublicKey)
            .await
            .expect("Gateway keypair does not exist")
    }

    async fn load_or_create_gateway_keypair(&mut self) -> Keypair {
        if let Some(key_pair) = self.get_value(&GatewayPublicKey).await {
            key_pair
        } else {
            let context = Secp256k1::new();
            let (secret_key, _public_key) = context.generate_keypair(&mut OsRng);
            let key_pair = Keypair::from_secret_key(&context, &secret_key);
            self.insert_new_entry(&GatewayPublicKey, &key_pair).await;
            key_pair
        }
    }

    async fn save_new_preimage_authentication(
        &mut self,
        payment_hash: sha256::Hash,
        preimage_auth: sha256::Hash,
    ) {
        self.insert_new_entry(&PreimageAuthentication { payment_hash }, &preimage_auth)
            .await;
    }

    async fn load_preimage_authentication(
        &mut self,
        payment_hash: sha256::Hash,
    ) -> Option<sha256::Hash> {
        self.get_value(&PreimageAuthentication { payment_hash })
            .await
    }

    async fn save_registered_incoming_contract(
        &mut self,
        federation_id: FederationId,
        incoming_amount: Amount,
        contract: IncomingContract,
    ) -> Option<RegisteredIncomingContract> {
        self.insert_entry(
            &RegisteredIncomingContractKey(contract.commitment.payment_image.clone()),
            &RegisteredIncomingContract {
                federation_id,
                incoming_amount_msats: incoming_amount.msats,
                contract,
            },
        )
        .await
    }

    async fn load_registered_incoming_contract(
        &mut self,
        payment_image: PaymentImage,
    ) -> Option<RegisteredIncomingContract> {
        self.get_value(&RegisteredIncomingContractKey(payment_image))
            .await
    }

    async fn dump_database(
        &mut self,
        prefix_names: Vec<String>,
    ) -> BTreeMap<String, Box<dyn erased_serde::Serialize + Send>> {
        let mut gateway_items: BTreeMap<String, Box<dyn erased_serde::Serialize + Send>> =
            BTreeMap::new();
        let filtered_prefixes = DbKeyPrefix::iter().filter(|f| {
            prefix_names.is_empty() || prefix_names.contains(&f.to_string().to_lowercase())
        });

        for table in filtered_prefixes {
            match table {
                DbKeyPrefix::FederationConfig => {
                    push_db_pair_items!(
                        self,
                        FederationIdKeyPrefix,
                        FederationIdKey,
                        FederationConfig,
                        gateway_items,
                        "Federation Config"
                    );
                }
                DbKeyPrefix::GatewayPublicKey => {
                    if let Some(public_key) = self.load_gateway_keypair().await {
                        gateway_items
                            .insert("Gateway Public Key".to_string(), Box::new(public_key));
                    }
                }
                _ => {}
            }
        }

        gateway_items
    }
}

#[repr(u8)]
#[derive(Clone, EnumIter, Debug)]
enum DbKeyPrefix {
    FederationConfig = 0x04,
    GatewayPublicKey = 0x06,
    GatewayConfiguration = 0x07,
    PreimageAuthentication = 0x08,
    RegisteredIncomingContract = 0x09,
    ClientDatabase = 0x10,
}

impl std::fmt::Display for DbKeyPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Encodable, Decodable)]
struct FederationIdKeyPrefixV0;

#[derive(Debug, Encodable, Decodable)]
struct FederationIdKeyPrefixV1;

#[derive(Debug, Encodable, Decodable)]
struct FederationIdKeyPrefix;

#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct FederationIdKeyV0 {
    id: FederationId,
}

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
pub struct FederationConfigV0 {
    pub invite_code: InviteCode,
    pub federation_index: u64,
    pub timelock_delta: u64,
    #[serde(with = "serde_routing_fees")]
    pub fees: RoutingFees,
}

#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct FederationIdKeyV1 {
    id: FederationId,
}

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
pub struct FederationConfigV1 {
    pub invite_code: InviteCode,
    // Unique integer identifier per-federation that is assigned when the gateways joins a
    // federation.
    #[serde(alias = "mint_channel_id")]
    pub federation_index: u64,
    pub timelock_delta: u64,
    #[serde(with = "serde_routing_fees")]
    pub fees: RoutingFees,
    pub connector: Connector,
}

#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct FederationIdKey {
    id: FederationId,
}

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
pub struct FederationConfig {
    pub invite_code: InviteCode,
    // Unique integer identifier per-federation that is assigned when the gateways joins a
    // federation.
    #[serde(alias = "mint_channel_id")]
    pub federation_index: u64,
    pub lightning_fee: PaymentFee,
    pub transaction_fee: PaymentFee,
    pub connector: Connector,
}

impl_db_record!(
    key = FederationIdKeyV0,
    value = FederationConfigV0,
    db_prefix = DbKeyPrefix::FederationConfig,
);

impl_db_record!(
    key = FederationIdKeyV1,
    value = FederationConfigV1,
    db_prefix = DbKeyPrefix::FederationConfig,
);

impl_db_record!(
    key = FederationIdKey,
    value = FederationConfig,
    db_prefix = DbKeyPrefix::FederationConfig,
);

impl_db_lookup!(
    key = FederationIdKeyV0,
    query_prefix = FederationIdKeyPrefixV0
);
impl_db_lookup!(
    key = FederationIdKeyV1,
    query_prefix = FederationIdKeyPrefixV1
);
impl_db_lookup!(key = FederationIdKey, query_prefix = FederationIdKeyPrefix);

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable)]
struct GatewayPublicKey;

impl_db_record!(
    key = GatewayPublicKey,
    value = Keypair,
    db_prefix = DbKeyPrefix::GatewayPublicKey,
);

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable)]
struct GatewayConfigurationKeyV0;

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
struct GatewayConfigurationV0 {
    password: String,
    num_route_hints: u32,
    #[serde(with = "serde_routing_fees")]
    routing_fees: RoutingFees,
    network: NetworkLegacyEncodingWrapper,
}

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable)]
pub struct GatewayConfigurationKeyV1;

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
pub struct GatewayConfigurationV1 {
    pub hashed_password: sha256::Hash,
    pub num_route_hints: u32,
    #[serde(with = "serde_routing_fees")]
    pub routing_fees: RoutingFees,
    pub network: NetworkLegacyEncodingWrapper,
    pub password_salt: [u8; 16],
}

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable)]
pub struct GatewayConfigurationKeyV2;

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable, Serialize, Deserialize)]
pub struct GatewayConfigurationV2 {
    pub num_route_hints: u32,
    #[serde(with = "serde_routing_fees")]
    pub routing_fees: RoutingFees,
    pub network: NetworkLegacyEncodingWrapper,
}

impl_db_record!(
    key = GatewayConfigurationKeyV0,
    value = GatewayConfigurationV0,
    db_prefix = DbKeyPrefix::GatewayConfiguration,
);

impl_db_record!(
    key = GatewayConfigurationKeyV1,
    value = GatewayConfigurationV1,
    db_prefix = DbKeyPrefix::GatewayConfiguration,
);

impl_db_record!(
    key = GatewayConfigurationKeyV2,
    value = GatewayConfigurationV2,
    db_prefix = DbKeyPrefix::GatewayConfiguration,
);

#[derive(Debug, Clone, Eq, PartialEq, Encodable, Decodable)]
struct PreimageAuthentication {
    payment_hash: sha256::Hash,
}

impl_db_record!(
    key = PreimageAuthentication,
    value = sha256::Hash,
    db_prefix = DbKeyPrefix::PreimageAuthentication
);

#[derive(Debug, Encodable, Decodable)]
struct PreimageAuthenticationPrefix;

impl_db_lookup!(
    key = PreimageAuthentication,
    query_prefix = PreimageAuthenticationPrefix
);

pub fn get_gatewayd_database_migrations() -> BTreeMap<DatabaseVersion, CoreMigrationFn> {
    let mut migrations: BTreeMap<DatabaseVersion, CoreMigrationFn> = BTreeMap::new();
    migrations.insert(DatabaseVersion(0), |ctx| migrate_to_v1(ctx).boxed());
    migrations.insert(DatabaseVersion(1), |ctx| migrate_to_v2(ctx).boxed());
    migrations.insert(DatabaseVersion(2), |ctx| migrate_to_v3(ctx).boxed());
    migrations.insert(DatabaseVersion(3), |ctx| migrate_to_v4(ctx).boxed());
    migrations.insert(DatabaseVersion(4), |ctx| migrate_to_v5(ctx).boxed());
    migrations
}

async fn migrate_to_v1(mut ctx: MigrationContext<'_>) -> Result<(), anyhow::Error> {
    /// Creates a password hash by appending a 4 byte salt to the plaintext
    /// password.
    fn hash_password(plaintext_password: &str, salt: [u8; 16]) -> sha256::Hash {
        let mut bytes = Vec::new();
        bytes.append(&mut plaintext_password.consensus_encode_to_vec());
        bytes.append(&mut salt.consensus_encode_to_vec());
        sha256::Hash::hash(&bytes)
    }

    let mut dbtx = ctx.dbtx();

    // If there is no old gateway configuration, there is nothing to do.
    if let Some(old_gateway_config) = dbtx.remove_entry(&GatewayConfigurationKeyV0).await {
        let password_salt: [u8; 16] = rand::thread_rng().gen();
        let hashed_password = hash_password(&old_gateway_config.password, password_salt);
        let new_gateway_config = GatewayConfigurationV1 {
            hashed_password,
            num_route_hints: old_gateway_config.num_route_hints,
            routing_fees: old_gateway_config.routing_fees,
            network: old_gateway_config.network,
            password_salt,
        };
        dbtx.insert_entry(&GatewayConfigurationKeyV1, &new_gateway_config)
            .await;
    }

    Ok(())
}

async fn migrate_to_v2(mut ctx: MigrationContext<'_>) -> Result<(), anyhow::Error> {
    let mut dbtx = ctx.dbtx();

    // If there is no old federation configuration, there is nothing to do.
    for (old_federation_id, _old_federation_config) in dbtx.load_federation_configs_v0().await {
        if let Some(old_federation_config) = dbtx
            .remove_entry(&FederationIdKeyV0 {
                id: old_federation_id,
            })
            .await
        {
            let new_federation_config = FederationConfigV1 {
                invite_code: old_federation_config.invite_code,
                federation_index: old_federation_config.federation_index,
                timelock_delta: old_federation_config.timelock_delta,
                fees: old_federation_config.fees,
                connector: Connector::default(),
            };
            let new_federation_key = FederationIdKeyV1 {
                id: old_federation_id,
            };
            dbtx.insert_entry(&new_federation_key, &new_federation_config)
                .await;
        }
    }
    Ok(())
}

async fn migrate_to_v3(mut ctx: MigrationContext<'_>) -> Result<(), anyhow::Error> {
    let mut dbtx = ctx.dbtx();

    // If there is no old gateway configuration, there is nothing to do.
    if let Some(old_gateway_config) = dbtx.remove_entry(&GatewayConfigurationKeyV1).await {
        let new_gateway_config = GatewayConfigurationV2 {
            num_route_hints: old_gateway_config.num_route_hints,
            routing_fees: old_gateway_config.routing_fees,
            network: old_gateway_config.network,
        };
        dbtx.insert_entry(&GatewayConfigurationKeyV2, &new_gateway_config)
            .await;
    }

    Ok(())
}

async fn migrate_to_v4(mut ctx: MigrationContext<'_>) -> Result<(), anyhow::Error> {
    let mut dbtx = ctx.dbtx();

    dbtx.remove_entry(&GatewayConfigurationKeyV2).await;

    let configs = dbtx
        .find_by_prefix(&FederationIdKeyPrefixV1)
        .await
        .collect::<Vec<_>>()
        .await;
    for (fed_id, _old_config) in configs {
        if let Some(old_federation_config) = dbtx.remove_entry(&fed_id).await {
            let new_fed_config = FederationConfig {
                invite_code: old_federation_config.invite_code,
                federation_index: old_federation_config.federation_index,
                lightning_fee: old_federation_config.fees.into(),
                transaction_fee: PaymentFee::TRANSACTION_FEE_DEFAULT,
                connector: Connector::default(),
            };
            let new_key = FederationIdKey { id: fed_id.id };
            dbtx.insert_new_entry(&new_key, &new_fed_config).await;
        }
    }
    Ok(())
}

async fn migrate_to_v5(mut ctx: MigrationContext<'_>) -> Result<(), anyhow::Error> {
    let mut dbtx = ctx.dbtx();
    migrate_federation_config(&mut dbtx).await
}

async fn migrate_federation_config(
    dbtx: &mut DatabaseTransaction<'_>,
) -> Result<(), anyhow::Error> {
    async fn migrate_client_entries(
        dbtx: &mut DatabaseTransaction<'_>,
        prefix: &[u8],
    ) -> Result<(), anyhow::Error> {
        let isolated_entries = dbtx
            .raw_find_by_prefix(prefix)
            .await?
            .collect::<BTreeMap<_, _>>()
            .await;
        for (mut key, value) in isolated_entries {
            dbtx.raw_remove_entry(&key).await?;
            let mut new_key = vec![DbKeyPrefix::ClientDatabase as u8];
            new_key.append(&mut key);
            dbtx.raw_insert_bytes(&new_key, &value).await?;
        }

        Ok(())
    }

    // We need to migrate all isolated database entries to be behind the 0x10
    // prefix. The problem is, if there is a `FederationId` that starts with
    // 0x04, we cannot read the `FederationId` because the database will be confused
    // between the isolated DB and the `FederationIdKey` record. To solve this,
    // we first try and see if there are any entries that begin with
    // 0x0404. This indicates a joined federation that will have a
    // problem decoding the ID.
    let problem_fed_configs = dbtx
        .raw_find_by_prefix(&[0x04, 0x04])
        .await?
        .collect::<BTreeMap<_, _>>()
        .await;
    for (problem_key, problem_fed_config) in problem_fed_configs {
        let federation_id = FederationId::consensus_decode_vec(
            problem_key[1..33].to_vec(),
            &ModuleDecoderRegistry::default(),
        )?;
        tracing::warn!(
            ?federation_id,
            "Found a FederationConfig entry that will cause issues decoding"
        );
        let federation_id_bytes = problem_key[1..3].to_vec();

        // Remove the `FederationConfig` entry so that the migration of the isolated
        // database doesn't accidentally migrate that too.
        dbtx.raw_remove_entry(&problem_key).await?;

        migrate_client_entries(dbtx, &federation_id_bytes).await?;

        // Re-insert the `FederationConfig` entry so that it exists after the migration.
        dbtx.raw_insert_bytes(&problem_key, &problem_fed_config)
            .await?;
    }

    // Migrate the rest of the isolated databases that don't overlap with
    // `FederationConfig`
    let fed_ids = dbtx
        .find_by_prefix(&FederationIdKeyPrefix)
        .await
        .collect::<BTreeMap<_, _>>()
        .await;
    for fed_id in fed_ids.keys() {
        let federation_id_bytes = fed_id.id.consensus_encode_to_vec();
        migrate_client_entries(dbtx, &federation_id_bytes).await?;
    }

    Ok(())
}

#[derive(Debug, Encodable, Decodable)]
struct RegisteredIncomingContractKey(pub PaymentImage);

#[derive(Debug, Encodable, Decodable)]
pub struct RegisteredIncomingContract {
    pub federation_id: FederationId,
    /// The amount of the incoming contract, in msats.
    pub incoming_amount_msats: u64,
    pub contract: IncomingContract,
}

impl_db_record!(
    key = RegisteredIncomingContractKey,
    value = RegisteredIncomingContract,
    db_prefix = DbKeyPrefix::RegisteredIncomingContract,
);

#[cfg(test)]
mod fedimint_migration_tests {
    use std::str::FromStr;

    use anyhow::ensure;
    use bitcoin::hashes::Hash;
    use fedimint_core::db::mem_impl::MemDatabase;
    use fedimint_core::db::Database;
    use fedimint_core::module::registry::ModuleDecoderRegistry;
    use fedimint_core::util::SafeUrl;
    use fedimint_core::PeerId;
    use fedimint_lnv2_common::gateway_api::PaymentFee;
    use fedimint_logging::TracingSetup;
    use fedimint_testing::db::{
        snapshot_db_migrations_with_decoders, validate_migrations_global, BYTE_32,
    };
    use strum::IntoEnumIterator;
    use tracing::info;

    use super::*;

    async fn create_gatewayd_db_data(db: Database) {
        let mut dbtx = db.begin_transaction().await;
        let federation_id = FederationId::dummy();
        let invite_code = InviteCode::new(
            SafeUrl::from_str("http://myexamplefed.com").expect("SafeUrl parsing can't fail"),
            0.into(),
            federation_id,
            None,
        );
        let federation_config = FederationConfigV0 {
            invite_code,
            federation_index: 2,
            timelock_delta: 10,
            fees: PaymentFee::TRANSACTION_FEE_DEFAULT.into(),
        };

        dbtx.insert_new_entry(&FederationIdKeyV0 { id: federation_id }, &federation_config)
            .await;

        let context = secp256k1::Secp256k1::new();
        let (secret, _) = context.generate_keypair(&mut OsRng);
        let key_pair = secp256k1::Keypair::from_secret_key(&context, &secret);
        dbtx.insert_new_entry(&GatewayPublicKey, &key_pair).await;

        let gateway_configuration = GatewayConfigurationV0 {
            password: "EXAMPLE".to_string(),
            num_route_hints: 2,
            routing_fees: PaymentFee::TRANSACTION_FEE_DEFAULT.into(),
            network: NetworkLegacyEncodingWrapper(bitcoin::Network::Regtest),
        };

        dbtx.insert_new_entry(&GatewayConfigurationKeyV0, &gateway_configuration)
            .await;

        let preimage_auth = PreimageAuthentication {
            payment_hash: sha256::Hash::from_slice(&BYTE_32).expect("Hash should not fail"),
        };
        let verification_hash = sha256::Hash::from_slice(&BYTE_32).expect("Hash should not fail");
        dbtx.insert_new_entry(&preimage_auth, &verification_hash)
            .await;

        dbtx.commit_tx().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn snapshot_server_db_migrations() -> anyhow::Result<()> {
        snapshot_db_migrations_with_decoders(
            "gatewayd",
            |db| {
                Box::pin(async {
                    create_gatewayd_db_data(db).await;
                })
            },
            ModuleDecoderRegistry::from_iter([]),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_server_db_migrations() -> anyhow::Result<()> {
        let _ = TracingSetup::default().init();
        validate_migrations_global(
            |db| async move {
                let mut dbtx = db.begin_transaction_nc().await;

                for prefix in DbKeyPrefix::iter() {
                    match prefix {
                        DbKeyPrefix::FederationConfig => {
                            let configs = dbtx
                                .find_by_prefix(&FederationIdKeyPrefix)
                                .await
                                .collect::<Vec<_>>()
                                .await;
                            let num_configs = configs.len();
                            ensure!(
                                num_configs > 0,
                                "validate_migrations was not able to read any FederationConfigs"
                            );
                            info!("Validated FederationConfig");
                        }
                        DbKeyPrefix::GatewayPublicKey => {
                            let gateway_id = dbtx.get_value(&GatewayPublicKey).await;
                            ensure!(gateway_id.is_some(), "validate_migrations was not able to read GatewayPublicKey");
                            info!("Validated GatewayPublicKey");
                        }
                        DbKeyPrefix::PreimageAuthentication => {
                            let preimage_authentications = dbtx.find_by_prefix(&PreimageAuthenticationPrefix).await.collect::<Vec<_>>().await;
                            let num_auths = preimage_authentications.len();
                            ensure!(num_auths > 0, "validate_migrations was not able to read any PreimageAuthentication");
                            info!("Validated PreimageAuthentication");
                        }
                        _ => {}
                    }
                }
                Ok(())
            },
            "gatewayd",
            get_gatewayd_database_migrations(),
            ModuleDecoderRegistry::from_iter([]),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_isolated_db_migration() -> anyhow::Result<()> {
        async fn create_isolated_record(prefix: Vec<u8>, db: &Database) {
            // Create an isolated database the old way where there was no prefix
            let isolated_db = db.with_prefix(prefix);
            let mut isolated_dbtx = isolated_db.begin_transaction().await;

            // Insert a record into the isolated db (doesn't matter what it is)
            isolated_dbtx
                .insert_new_entry(
                    &GatewayPublicKey,
                    &Keypair::new(secp256k1::SECP256K1, &mut rand::thread_rng()),
                )
                .await;
            isolated_dbtx.commit_tx().await;
        }

        let federation_id = FederationId::from_str(
            "0406afdc71a052d2787eab7e84c95803636d2a84c272eb81b4e01b27acb86c6f",
        )
        .expect("invalid federation ID");
        let _ = TracingSetup::default().init();
        let db = Database::new(MemDatabase::new(), ModuleDecoderRegistry::default());
        let mut dbtx = db.begin_transaction().await;
        dbtx.insert_new_entry(
            &FederationIdKey { id: federation_id },
            &FederationConfig {
                invite_code: InviteCode::new(
                    SafeUrl::from_str("http://testfed.com").unwrap(),
                    PeerId::from(0),
                    federation_id,
                    None,
                ),
                federation_index: 0,
                lightning_fee: PaymentFee::TRANSACTION_FEE_DEFAULT,
                transaction_fee: PaymentFee::TRANSACTION_FEE_DEFAULT,
                connector: Connector::Tcp,
            },
        )
        .await;

        dbtx.insert_new_entry(
            &FederationIdKey {
                id: FederationId::dummy(),
            },
            &FederationConfig {
                invite_code: InviteCode::new(
                    SafeUrl::from_str("http://testfed2.com").unwrap(),
                    PeerId::from(0),
                    FederationId::dummy(),
                    None,
                ),
                federation_index: 1,
                lightning_fee: PaymentFee::TRANSACTION_FEE_DEFAULT,
                transaction_fee: PaymentFee::TRANSACTION_FEE_DEFAULT,
                connector: Connector::Tcp,
            },
        )
        .await;
        dbtx.commit_tx().await;

        create_isolated_record(federation_id.consensus_encode_to_vec(), &db).await;
        create_isolated_record(FederationId::dummy().consensus_encode_to_vec(), &db).await;

        let mut migration_dbtx = db.begin_transaction().await;
        migrate_federation_config(&mut migration_dbtx.to_ref_nc()).await?;
        migration_dbtx.commit_tx().await;

        let mut dbtx = db.begin_transaction_nc().await;

        let num_configs = dbtx
            .find_by_prefix(&FederationIdKeyPrefix)
            .await
            .collect::<BTreeMap<_, _>>()
            .await
            .len();
        assert_eq!(num_configs, 2);

        // Verify that the client databases migrated successfully.
        let isolated_db = db.get_client_database(&federation_id);
        let mut isolated_dbtx = isolated_db.begin_transaction_nc().await;
        assert!(isolated_dbtx.get_value(&GatewayPublicKey).await.is_some());

        let isolated_db = db.get_client_database(&FederationId::dummy());
        let mut isolated_dbtx = isolated_db.begin_transaction_nc().await;
        assert!(isolated_dbtx.get_value(&GatewayPublicKey).await.is_some());

        Ok(())
    }
}
