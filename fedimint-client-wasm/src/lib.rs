#![cfg(target_family = "wasm")]
mod db;

use std::pin::pin;
use std::str::FromStr;
use std::sync::Arc;

use async_stream::try_stream;
use db::MemAndIndexedDb;
use fedimint_client::module::IClientModule;
use fedimint_client::secret::{PlainRootSecretStrategy, RootSecretStrategy};
use fedimint_client::ClientHandleArc;
use fedimint_core::db::Database;
use fedimint_core::invite_code::InviteCode;
use fedimint_ln_client::{LightningClientInit, LightningClientModule};
use fedimint_mint_client::MintClientInit;
use futures::StreamExt;
use serde_json::json;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub struct WasmClient {
    client: ClientHandleArc,
}

#[wasm_bindgen]
impl WasmClient {
    #[wasm_bindgen]
    /// Open fedimint client with already joined federation.
    ///
    /// After you have joined a federation, you can reopen the fedimint client
    /// with same client_name. Opening client with same name at same time is
    /// not supported. You can close the current client by calling
    /// `client.free()`. NOTE: The client will remain active until all the
    /// running rpc calls have finished.
    // WasmClient::free is auto generated by wasm bindgen.
    pub async fn open(client_name: String) -> Result<Option<WasmClient>, JsError> {
        Self::open_inner(client_name)
            .await
            .map_err(|x| JsError::new(&x.to_string()))
    }

    #[wasm_bindgen]
    /// Open a fedimint client by join a federation.
    pub async fn join_federation(
        client_name: String,
        invite_code: String,
    ) -> Result<WasmClient, JsError> {
        Self::join_federation_inner(client_name, invite_code)
            .await
            .map_err(|x| JsError::new(&x.to_string()))
    }

    async fn client_builder(db: Database) -> Result<fedimint_client::ClientBuilder, anyhow::Error> {
        let mut builder = fedimint_client::Client::builder(db).await?;
        builder.with_module(MintClientInit);
        builder.with_module(LightningClientInit::default());
        // FIXME: wallet module?
        builder.with_primary_module(1);
        Ok(builder)
    }

    async fn open_inner(client_name: String) -> anyhow::Result<Option<WasmClient>> {
        let db = Database::from(MemAndIndexedDb::new(&client_name).await?);
        if !fedimint_client::Client::is_initialized(&db).await {
            return Ok(None);
        }
        let client_secret = fedimint_client::Client::load_or_generate_client_secret(&db).await?;
        let root_secret = PlainRootSecretStrategy::to_root_secret(&client_secret);
        let builder = Self::client_builder(db).await?;
        Ok(Some(Self {
            client: Arc::new(builder.open(root_secret).await?),
        }))
    }

    async fn join_federation_inner(
        client_name: String,
        invite_code: String,
    ) -> anyhow::Result<WasmClient> {
        let db = Database::from(MemAndIndexedDb::new(&client_name).await?);
        let client_secret = fedimint_client::Client::load_or_generate_client_secret(&db).await?;
        let root_secret = PlainRootSecretStrategy::to_root_secret(&client_secret);
        let builder = Self::client_builder(db).await?;
        let invite_code = InviteCode::from_str(&invite_code)?;
        let config = fedimint_api_client::api::net::Connector::default()
            .download_from_invite_code(&invite_code)
            .await?;
        let client = Arc::new(builder.join(root_secret, config, None).await?);
        Ok(Self { client })
    }

    #[wasm_bindgen]
    /// Call a fedimint client rpc the responses are returned using `cb`
    /// callback. Each rpc call *can* return multiple responses by calling
    /// `cb` multiple times. You should ignore the promise by this function
    /// because it has no significance.
    pub async fn rpc(&self, module: &str, method: &str, payload: String, cb: &js_sys::Function) {
        let mut stream = pin!(self.rpc_inner(module, method, payload));

        while let Some(item) = stream.next().await {
            let this = JsValue::null();
            let _ = match item {
                Ok(item) => cb.call1(
                    &this,
                    &JsValue::from_str(&serde_json::to_string(&item).unwrap()),
                ),
                Err(err) => cb.call1(
                    &this,
                    &JsValue::from_str(
                        &serde_json::to_string(&json!({"error": err.to_string()})).unwrap(),
                    ),
                ),
            };
        }
    }

    fn rpc_inner<'a>(
        &'a self,
        module: &'a str,
        method: &'a str,
        payload: String,
    ) -> impl futures::Stream<Item = anyhow::Result<serde_json::Value>> + 'a {
        try_stream! {
            let payload: serde_json::Value = serde_json::from_str(&payload)?;
            match module {
                "" => {
                    let mut stream = self.client.handle_global_rpc(method.to_owned(), payload);
                    while let Some(item) = stream.next().await {
                        yield item?;
                    }
                }
                "ln" => {
                    let ln = self
                        .client
                        .get_first_module::<LightningClientModule>()?
                        .inner();
                    let mut stream = ln.handle_rpc(method.to_owned(), payload).await;
                    while let Some(item) = stream.next().await {
                        yield item?;
                    }
                }
                "mint" => {
                    let mint = self
                        .client
                        .get_first_module::<fedimint_mint_client::MintClientModule>()?
                        .inner();
                    let mut stream = mint.handle_rpc(method.to_owned(), payload).await;
                    while let Some(item) = stream.next().await {
                        yield item?;
                    }
                }
                _ => {
                    Err(anyhow::format_err!("module not found: {module}"))?;
                    unreachable!()
                },
            }
        }
    }
}
