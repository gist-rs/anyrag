## `Cargo.toml`

```toml
[workspace]
members = [
    "jupiter-swap-api-client",
    "example",
]
resolver = "2"

[workspace.package]
edition = "2021"

[workspace.dependencies]
solana-sdk = "2"
solana-client = "2"
solana-account-decoder = "2"

```
---
## `example/Cargo.toml`

```toml
[package]
name = "example"
version = "0.1.0"
description = ""
edition = { workspace = true }

[dependencies]
tokio = { version = "1", features = ["full"] }
jupiter-swap-api-client = { path = "../jupiter-swap-api-client" }
solana-sdk = { workspace = true }
solana-client = { workspace = true }
bincode = "1.3.3"
```
---
## `example/src/main.rs`

```rust
use std::env;

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey, transaction::VersionedTransaction};
use solana_sdk::{pubkey::Pubkey, signature::NullSigner};

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

pub const TEST_WALLET: Pubkey = pubkey!("2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm"); // Coinbase 2 wallet

#[tokio::main]
async fn main() {
    let api_base_url = env::var("API_BASE_URL").unwrap_or("https://quote-api.jup.ag/v6".into());
    println!("Using base url: {}", api_base_url);

    let jupiter_swap_api_client = JupiterSwapApiClient::new(api_base_url);

    let quote_request = QuoteRequest {
        amount: 1_000_000,
        input_mint: USDC_MINT,
        output_mint: NATIVE_MINT,
        dexes: Some("Whirlpool,Meteora DLMM,Raydium CLMM".into()),
        slippage_bps: 50,
        ..QuoteRequest::default()
    };

    // GET /quote
    let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();
    println!("{quote_response:#?}");

    // POST /swap
    let swap_response = jupiter_swap_api_client
        .swap(
            &SwapRequest {
                user_public_key: TEST_WALLET,
                quote_response: quote_response.clone(),
                config: TransactionConfig::default(),
            },
            None,
        )
        .await
        .unwrap();

    println!("Raw tx len: {}", swap_response.swap_transaction.len());

    let versioned_transaction: VersionedTransaction =
        bincode::deserialize(&swap_response.swap_transaction).unwrap();

    // Replace with a keypair or other struct implementing signer
    let null_signer = NullSigner::new(&TEST_WALLET);
    let signed_versioned_transaction =
        VersionedTransaction::try_new(versioned_transaction.message, &[&null_signer]).unwrap();

    // send with rpc client...
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".into());

    // This will fail with "Transaction signature verification failure" as we did not really sign
    let error = rpc_client
        .send_and_confirm_transaction(&signed_versioned_transaction)
        .await
        .unwrap_err();
    println!("{error}");

    // POST /swap-instructions
    let swap_instructions = jupiter_swap_api_client
        .swap_instructions(&SwapRequest {
            user_public_key: TEST_WALLET,
            quote_response,
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();
    println!("swap_instructions: {swap_instructions:?}");
}

```
---
## `README.md`

```markdown
# jup-swap-api-client

## Introduction

The `jup-swap-api-client` is a Rust client library designed to simplify the integration of the Jupiter Swap API, enabling seamless swaps on the Solana blockchain.

## Getting Started

To use the `jup-swap-api-client` crate in your Rust project, follow these simple steps:

Add the crate to your `Cargo.toml`:

    ```toml
    [dependencies]
    jupiter-swap-api-client = { git = "https://github.com/jup-ag/jupiter-swap-api-client.git", package = "jupiter-swap-api-client"}
    ```

## Examples

Here's a simplified example of how to use the `jup-swap-api-client` in your Rust application:

```rust
use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_sdk::pubkey::Pubkey;

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
const TEST_WALLET: Pubkey = pubkey!("2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm");

#[tokio::main]
async fn main() {
    let jupiter_swap_api_client = JupiterSwapApiClient::new("https://quote-api.jup.ag/v6");

    let quote_request = QuoteRequest {
        amount: 1_000_000,
        input_mint: USDC_MINT,
        output_mint: NATIVE_MINT,
        slippage_bps: 50,
        ..QuoteRequest::default()
    };

    // GET /quote
    let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();
    println!("{quote_response:#?}");

    // POST /swap
    let swap_response = jupiter_swap_api_client
        .swap(&SwapRequest {
            user_public_key: TEST_WALLET,
            quote_response: quote_response.clone(),
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();

    println!("Raw tx len: {}", swap_response.swap_transaction.len());

    // Perform further actions as needed...

    // POST /swap-instructions
    let swap_instructions = jupiter_swap_api_client
        .swap_instructions(&SwapRequest {
            user_public_key: TEST_WALLET,
            quote_response,
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();
    println!("{swap_instructions:#?}");
}

```
For the full example, please refer to the [examples](./example/) directory in this repository.

### Using Self-hosted APIs

You can set custom URLs via environment variables for any self-hosted Jupiter APIs. Like the [V6 Swap API](https://station.jup.ag/docs/apis/self-hosted) or the [paid hosted APIs](#paid-hosted-apis). Here are the ENV vars:

```
API_BASE_URL=https://hosted.api
```

### Paid Hosted APIs

You can also check out some of the [paid hosted APIs](https://station.jup.ag/docs/apis/self-hosted#paid-hosted-apis).

## Additional Resources

- [Jupiter Swap API Documentation](https://station.jup.ag/docs/v6/swap-api): Learn more about the Jupiter Swap API and its capabilities.
- [jup.ag Website](https://jup.ag/): Explore the official website for additional information and resources.

```
---
## `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.87.0"
```
---
## `jupiter-swap-api-client/Cargo.toml`

```toml
[package]
name = "jupiter-swap-api-client"
version = "0.2.0"
description = "Jupiter Swap API rust client"
license = "Apache-2.0"
edition = { workspace = true }

[dependencies]
anyhow = "1"
serde = { version = "1.0.159", features = ["derive"] }
serde_json = "1.0.95"
solana-sdk = { workspace = true }
solana-account-decoder = { workspace = true }
thiserror = "2"
base64 = "0.22.1"
serde_qs = "0.13.0"
reqwest = { version = "0.12", features = ["json"] }
rust_decimal = "1.36.0"

```
---
## `jupiter-swap-api-client/src/quote.rs`

```rust
//! Quote data structure for quoting and quote response
//!

use std::{collections::HashMap, str::FromStr};

use crate::route_plan_with_metadata::RoutePlanWithMetadata;
use crate::serde_helpers::field_as_string;
use anyhow::{anyhow, Error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Swap information of each Swap occurred in the route paths
pub struct SwapInfo {
    #[serde(with = "field_as_string")]
    pub amm_key: Pubkey,
    pub label: String,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    /// An estimation of the input amount into the AMM
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    /// An estimation of the output amount into the AMM
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_mint: Pubkey,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

impl FromStr for SwapMode {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ExactIn" => Ok(Self::ExactIn),
            "ExactOut" => Ok(Self::ExactOut),
            _ => Err(anyhow!("{} is not a valid SwapMode", s)),
        }
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct ComputeUnitScore {
    pub max_penalty_bps: Option<f64>,
}

#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    /// The amount to swap, have to factor in the token decimals.
    #[serde(with = "field_as_string")]
    pub amount: u64,
    /// (ExactIn or ExactOut) Defaults to ExactIn.
    /// ExactOut is for supporting use cases where you need an exact token amount, like payments.
    /// In this case the slippage is on the input token.
    pub swap_mode: Option<SwapMode>,
    /// Allowed slippage in basis points
    pub slippage_bps: u16,
    /// Default is false.
    /// By setting this to true, our API will suggest smart slippage info that you can use.
    /// slippageBps is what we suggest you to use. Additionally, you should check out max_auto_slippage_bps and auto_slippage_collision_usd_value.
    pub auto_slippage: Option<bool>,
    /// The max amount of slippage in basis points that you are willing to accept for auto slippage.
    pub max_auto_slippage_bps: Option<u16>,
    pub compute_auto_slippage: bool,
    /// The max amount of USD value that you are willing to accept for auto slippage.
    pub auto_slippage_collision_usd_value: Option<u32>,
    /// Quote with a greater amount to find the route to minimize slippage
    pub minimize_slippage: Option<bool>,
    /// Platform fee in basis points
    pub platform_fee_bps: Option<u8>,
    pub dexes: Option<Dexes>,
    pub excluded_dexes: Option<Dexes>,
    /// Quote only direct routes
    pub only_direct_routes: Option<bool>,
    /// Quote fit into legacy transaction
    pub as_legacy_transaction: Option<bool>,
    /// Restrict intermediate tokens to a top token set that has stable liquidity.
    /// This will help to ease potential high slippage error rate when swapping with minimal impact on pricing.
    pub restrict_intermediate_tokens: Option<bool>,
    /// Find a route given a maximum number of accounts involved,
    /// this might dangerously limit routing ending up giving a bad price.
    /// The max is an estimation and not the exact count
    pub max_accounts: Option<usize>,
    /// Quote type to be used for routing, switches the algorithm
    pub quote_type: Option<String>,
    /// Extra args which are quote type specific to allow controlling settings from the top level
    pub quote_args: Option<HashMap<String, String>>,
    /// enable only full liquid markets as intermediate tokens
    pub prefer_liquid_dexes: Option<bool>,
    /// Use the compute unit score to pick a route
    pub compute_unit_score: Option<ComputeUnitScore>,
    /// Routing constraints
    pub routing_constraints: Option<String>,
    /// Token category based intermediates token
    pub token_category_based_intermediate_tokens: Option<bool>,
}

// Essentially the same as QuoteRequest, but without the extra args
// as we pass the extra args separately
#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InternalQuoteRequest {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    /// The amount to swap, have to factor in the token decimals.
    #[serde(with = "field_as_string")]
    pub amount: u64,
    /// (ExactIn or ExactOut) Defaults to ExactIn.
    /// ExactOut is for supporting use cases where you need an exact token amount, like payments.
    /// In this case the slippage is on the input token.
    pub swap_mode: Option<SwapMode>,
    /// Allowed slippage in basis points
    pub slippage_bps: u16,
    /// Default is false.
    /// By setting this to true, our API will suggest smart slippage info that you can use.
    /// slippageBps is what we suggest you to use. Additionally, you should check out max_auto_slippage_bps and auto_slippage_collision_usd_value.
    pub auto_slippage: Option<bool>,
    /// The max amount of slippage in basis points that you are willing to accept for auto slippage.
    pub max_auto_slippage_bps: Option<u16>,
    pub compute_auto_slippage: bool,
    /// The max amount of USD value that you are willing to accept for auto slippage.
    pub auto_slippage_collision_usd_value: Option<u32>,
    /// Quote with a greater amount to find the route to minimize slippage
    pub minimize_slippage: Option<bool>,
    /// Platform fee in basis points
    pub platform_fee_bps: Option<u8>,
    pub dexes: Option<Dexes>,
    pub excluded_dexes: Option<Dexes>,
    /// Quote only direct routes
    pub only_direct_routes: Option<bool>,
    /// Quote fit into legacy transaction
    pub as_legacy_transaction: Option<bool>,
    /// Restrict intermediate tokens to a top token set that has stable liquidity.
    /// This will help to ease potential high slippage error rate when swapping with minimal impact on pricing.
    pub restrict_intermediate_tokens: Option<bool>,
    /// Find a route given a maximum number of accounts involved,
    /// this might dangerously limit routing ending up giving a bad price.
    /// The max is an estimation and not the exact count
    pub max_accounts: Option<usize>,
    // Quote type to be used for routing, switches the algorithm
    pub quote_type: Option<String>,
    // enable only full liquid markets as intermediate tokens
    pub prefer_liquid_dexes: Option<bool>,
}

impl From<QuoteRequest> for InternalQuoteRequest {
    fn from(request: QuoteRequest) -> Self {
        InternalQuoteRequest {
            input_mint: request.input_mint,
            output_mint: request.output_mint,
            amount: request.amount,
            swap_mode: request.swap_mode,
            slippage_bps: request.slippage_bps,
            auto_slippage: request.auto_slippage,
            max_auto_slippage_bps: request.max_auto_slippage_bps,
            compute_auto_slippage: request.compute_auto_slippage,
            auto_slippage_collision_usd_value: request.auto_slippage_collision_usd_value,
            minimize_slippage: request.minimize_slippage,
            platform_fee_bps: request.platform_fee_bps,
            dexes: request.dexes,
            excluded_dexes: request.excluded_dexes,
            only_direct_routes: request.only_direct_routes,
            as_legacy_transaction: request.as_legacy_transaction,
            restrict_intermediate_tokens: request.restrict_intermediate_tokens,
            max_accounts: request.max_accounts,
            quote_type: request.quote_type,
            prefer_liquid_dexes: request.prefer_liquid_dexes,
        }
    }
}

/// Comma delimited list of dex labels
type Dexes = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFee {
    #[serde(with = "field_as_string")]
    pub amount: u64,
    pub fee_bps: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    /// Not used by build transaction
    #[serde(with = "field_as_string")]
    pub other_amount_threshold: u64,
    pub swap_mode: SwapMode,
    pub slippage_bps: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed_auto_slippage: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_quote_minimizing_slippage: Option<bool>,
    pub platform_fee: Option<PlatformFee>,
    pub price_impact_pct: Decimal,
    pub route_plan: RoutePlanWithMetadata,
    #[serde(default)]
    pub context_slot: u64,
    #[serde(default)]
    pub time_taken: f64,
}

```
---
## `jupiter-swap-api-client/src/swap.rs`

```rust
use crate::{
    quote::QuoteResponse, serde_helpers::field_as_string, transaction_config::TransactionConfig,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    #[serde(with = "field_as_string")]
    pub user_public_key: Pubkey,
    pub quote_response: QuoteResponse,
    #[serde(flatten)]
    pub config: TransactionConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PrioritizationType {
    #[serde(rename_all = "camelCase")]
    Jito { lamports: u64 },
    #[serde(rename_all = "camelCase")]
    ComputeBudget {
        micro_lamports: u64,
        estimated_micro_lamports: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DynamicSlippageReport {
    pub slippage_bps: u16,
    pub other_amount: Option<u64>,
    /// Signed to convey positive and negative slippage
    pub simulated_incurred_slippage_bps: Option<i16>,
    pub amplification_ratio: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UiSimulationError {
    error_code: String,
    error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    #[serde(with = "base64_serialize_deserialize")]
    pub swap_transaction: Vec<u8>,
    pub last_valid_block_height: u64,
    pub prioritization_fee_lamports: u64,
    pub compute_unit_limit: u32,
    pub prioritization_type: Option<PrioritizationType>,
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    pub simulation_error: Option<UiSimulationError>,
}

pub mod base64_serialize_deserialize {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{de, Deserializer, Serializer};

    use super::*;
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base58 = STANDARD.encode(v);
        String::serialize(&base58, s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let field_string = String::deserialize(deserializer)?;
        STANDARD
            .decode(field_string)
            .map_err(|e| de::Error::custom(format!("base64 decoding error: {:?}", e)))
    }
}

#[derive(Debug, Clone)]
pub struct SwapInstructionsResponse {
    pub token_ledger_instruction: Option<Instruction>,
    pub compute_budget_instructions: Vec<Instruction>,
    pub setup_instructions: Vec<Instruction>,
    /// Instruction performing the action of swapping
    pub swap_instruction: Instruction,
    pub cleanup_instruction: Option<Instruction>,
    /// Other instructions that should be included in the transaction.
    /// Now, it should only have the Jito tip instruction.
    pub other_instructions: Vec<Instruction>,
    pub address_lookup_table_addresses: Vec<Pubkey>,
    pub prioritization_fee_lamports: u64,
    pub compute_unit_limit: u32,
    pub prioritization_type: Option<PrioritizationType>,
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    pub simulation_error: Option<UiSimulationError>,
}

// Duplicate for deserialization
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapInstructionsResponseInternal {
    token_ledger_instruction: Option<InstructionInternal>,
    compute_budget_instructions: Vec<InstructionInternal>,
    setup_instructions: Vec<InstructionInternal>,
    /// Instruction performing the action of swapping
    swap_instruction: InstructionInternal,
    cleanup_instruction: Option<InstructionInternal>,
    /// Other instructions that should be included in the transaction.
    /// Now, it should only have the Jito tip instruction.
    other_instructions: Vec<InstructionInternal>,
    address_lookup_table_addresses: Vec<PubkeyInternal>,
    prioritization_fee_lamports: u64,
    compute_unit_limit: u32,
    prioritization_type: Option<PrioritizationType>,
    dynamic_slippage_report: Option<DynamicSlippageReport>,
    simulation_error: Option<UiSimulationError>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct InstructionInternal {
    #[serde(with = "field_as_string")]
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMetaInternal>,
    #[serde(with = "base64_serialize_deserialize")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountMetaInternal {
    #[serde(with = "field_as_string")]
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<AccountMetaInternal> for AccountMeta {
    fn from(val: AccountMetaInternal) -> Self {
        AccountMeta {
            pubkey: val.pubkey,
            is_signer: val.is_signer,
            is_writable: val.is_writable,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct PubkeyInternal(#[serde(with = "field_as_string")] Pubkey);

impl From<InstructionInternal> for Instruction {
    fn from(val: InstructionInternal) -> Self {
        Instruction {
            program_id: val.program_id,
            accounts: val.accounts.into_iter().map(Into::into).collect(),
            data: val.data,
        }
    }
}

impl From<SwapInstructionsResponseInternal> for SwapInstructionsResponse {
    fn from(value: SwapInstructionsResponseInternal) -> Self {
        Self {
            token_ledger_instruction: value.token_ledger_instruction.map(Into::into),
            compute_budget_instructions: value
                .compute_budget_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            setup_instructions: value
                .setup_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            swap_instruction: value.swap_instruction.into(),
            cleanup_instruction: value.cleanup_instruction.map(Into::into),
            other_instructions: value
                .other_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            address_lookup_table_addresses: value
                .address_lookup_table_addresses
                .into_iter()
                .map(|p| p.0)
                .collect(),
            prioritization_fee_lamports: value.prioritization_fee_lamports,
            compute_unit_limit: value.compute_unit_limit,
            prioritization_type: value.prioritization_type,
            dynamic_slippage_report: value.dynamic_slippage_report,
            simulation_error: value.simulation_error,
        }
    }
}

```
---
## `jupiter-swap-api-client/src/lib.rs`

```rust
use std::collections::HashMap;

use quote::{InternalQuoteRequest, QuoteRequest, QuoteResponse};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use swap::{SwapInstructionsResponse, SwapInstructionsResponseInternal, SwapRequest, SwapResponse};
use thiserror::Error;

pub mod quote;
pub mod route_plan_with_metadata;
pub mod serde_helpers;
pub mod swap;
pub mod transaction_config;

#[derive(Clone)]
pub struct JupiterSwapApiClient {
    pub base_path: String,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Request failed with status {status}: {body}")]
    RequestFailed {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Failed to deserialize response: {0}")]
    DeserializationError(#[from] reqwest::Error),
}

async fn check_is_success(response: Response) -> Result<Response, ClientError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::RequestFailed { status, body });
    }
    Ok(response)
}

async fn check_status_code_and_deserialize<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ClientError> {
    let response = check_is_success(response).await?;
    response
        .json::<T>()
        .await
        .map_err(ClientError::DeserializationError)
}

impl JupiterSwapApiClient {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    pub async fn quote(&self, quote_request: &QuoteRequest) -> Result<QuoteResponse, ClientError> {
        let url = format!("{}/quote", self.base_path);
        let extra_args = quote_request.quote_args.clone();
        let internal_quote_request = InternalQuoteRequest::from(quote_request.clone());
        let response = Client::new()
            .get(url)
            .query(&internal_quote_request)
            .query(&extra_args)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap(
        &self,
        swap_request: &SwapRequest,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<SwapResponse, ClientError> {
        let response = Client::new()
            .post(format!("{}/swap", self.base_path))
            .query(&extra_args)
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap_instructions(
        &self,
        swap_request: &SwapRequest,
    ) -> Result<SwapInstructionsResponse, ClientError> {
        let response = Client::new()
            .post(format!("{}/swap-instructions", self.base_path))
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize::<SwapInstructionsResponseInternal>(response)
            .await
            .map(Into::into)
    }
}

```
---
## `jupiter-swap-api-client/src/route_plan_with_metadata.rs`

```rust
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::serde_helpers::field_as_string;

/// Topologically sorted DAG with additional metadata for rendering
pub type RoutePlanWithMetadata = Vec<RoutePlanStep>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    #[serde(with = "field_as_string")]
    pub amm_key: Pubkey,
    pub label: String,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    /// An estimation of the input amount into the AMM
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    /// An estimation of the output amount into the AMM
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_mint: Pubkey,
}

```
---
## `jupiter-swap-api-client/src/serde_helpers/option_field_as_string.rs`

```rust
use {
    serde::{de, Deserialize, Deserializer, Serialize, Serializer},
    std::str::FromStr,
};

pub fn serialize<T, S>(t: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    if let Some(t) = t {
        t.to_string().serialize(serializer)
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => s
            .parse()
            .map(Some)
            .map_err(|e| de::Error::custom(format!("Parse error: {:?}", e))),
        None => Ok(None),
    }
}

```
---
## `jupiter-swap-api-client/src/serde_helpers/mod.rs`

```rust
pub mod field_as_string;
pub mod option_field_as_string;

```
---
## `jupiter-swap-api-client/src/serde_helpers/field_as_string.rs`

```rust
use {
    serde::{de, Deserializer, Serializer},
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};

pub fn serialize<T, S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    t.to_string().serialize(serializer)
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let s: String = String::deserialize(deserializer)?;
    s.parse()
        .map_err(|e| de::Error::custom(format!("Parse error: {:?}", e)))
}

```
---
## `jupiter-swap-api-client/src/transaction_config.rs`

```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use solana_account_decoder::UiAccount;
use solana_sdk::pubkey::Pubkey;

use crate::serde_helpers::option_field_as_string;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ComputeUnitPriceMicroLamports {
    MicroLamports(u64),
    #[serde(deserialize_with = "auto")]
    Auto,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PriorityLevel {
    Medium,
    High,
    VeryHigh,
}

#[derive(Deserialize, Debug, PartialEq, Copy, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum PrioritizationFeeLamports {
    AutoMultiplier(u32),
    JitoTipLamports(u64),
    #[serde(rename_all = "camelCase")]
    PriorityLevelWithMaxLamports {
        priority_level: PriorityLevel,
        max_lamports: u64,
        #[serde(default)]
        global: bool,
    },
    #[default]
    #[serde(untagged, deserialize_with = "auto")]
    Auto,
    #[serde(untagged)]
    Lamports(u64),
    #[serde(untagged, deserialize_with = "disabled")]
    Disabled,
}

impl Serialize for PrioritizationFeeLamports {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct AutoMultiplier {
            auto_multiplier: u32,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct PriorityLevelWrapper<'a> {
            priority_level_with_max_lamports: PriorityLevelWithMaxLamports<'a>,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct PriorityLevelWithMaxLamports<'a> {
            priority_level: &'a PriorityLevel,
            max_lamports: &'a u64,
            global: &'a bool,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct JitoTipLamports {
            jito_tip_lamports: u64,
        }

        match self {
            Self::AutoMultiplier(auto_multiplier) => AutoMultiplier {
                auto_multiplier: *auto_multiplier,
            }
            .serialize(serializer),
            Self::JitoTipLamports(lamports) => JitoTipLamports {
                jito_tip_lamports: *lamports,
            }
            .serialize(serializer),
            Self::Auto => serializer.serialize_str("auto"),
            Self::Lamports(lamports) => serializer.serialize_u64(*lamports),
            Self::Disabled => serializer.serialize_str("disabled"),
            Self::PriorityLevelWithMaxLamports {
                priority_level,
                max_lamports,
                global,
            } => PriorityLevelWrapper {
                priority_level_with_max_lamports: PriorityLevelWithMaxLamports {
                    priority_level,
                    max_lamports,
                    global,
                },
            }
            .serialize(serializer),
        }
    }
}

fn auto<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "auto")]
        Variant,
    }
    Helper::deserialize(deserializer)?;
    Ok(())
}

fn disabled<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "disabled")]
        Variant,
    }
    Helper::deserialize(deserializer)?;
    Ok(())
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DynamicSlippageSettings {
    pub min_bps: Option<u16>,
    pub max_bps: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct TransactionConfig {
    /// Wrap and unwrap SOL. Will be ignored if `destination_token_account` is set because the `destination_token_account` may belong to a different user that we have no authority to close.
    pub wrap_and_unwrap_sol: bool,
    /// Allow optimized WSOL token account by using transfer, assign with seed, allocate with seed then initialize account 3 instead of the expensive associated token account process
    pub allow_optimized_wrapped_sol_token_account: bool,
    /// Fee token account for the output token, it is derived using the seeds = ["referral_ata", referral_account, mint] and the `REFER4ZgmyYx9c6He5XfaTMiGfdLwRnkV4RPp9t9iF3` referral contract (only pass in if you set a feeBps and make sure that the feeAccount has been created)
    #[serde(with = "option_field_as_string")]
    pub fee_account: Option<Pubkey>,
    /// Public key of the token account that will be used to receive the token out of the swap. If not provided, the user's ATA will be used. If provided, we assume that the token account is already initialized.
    #[serde(with = "option_field_as_string")]
    pub destination_token_account: Option<Pubkey>,
    /// Add a readonly, non signer tracking account that isn't used by jupiter
    #[serde(with = "option_field_as_string")]
    pub tracking_account: Option<Pubkey>,
    /// compute unit price to prioritize the transaction, the additional fee will be compute unit consumed * computeUnitPriceMicroLamports
    pub compute_unit_price_micro_lamports: Option<ComputeUnitPriceMicroLamports>,
    /// Prioritization fee lamports paid for the transaction in addition to the signatures fee.
    /// Mutually exclusive with `compute_unit_price_micro_lamports`.
    pub prioritization_fee_lamports: Option<PrioritizationFeeLamports>,
    /// When enabled, it will do a swap simulation to get the compute unit used and set it in ComputeBudget's compute unit limit.
    /// This will increase latency slightly since there will be one extra RPC call to simulate this. Default is false.
    pub dynamic_compute_unit_limit: bool,
    /// Request a legacy transaction rather than the default versioned transaction, needs to be paired with a quote using asLegacyTransaction otherwise the transaction might be too large
    ///
    /// Default: false
    pub as_legacy_transaction: bool,
    /// This enables the usage of shared program accounts. That means no intermediate token accounts or open orders accounts need to be created.
    /// But it also means that the likelihood of hot accounts is higher.
    ///
    /// Default: Optimized internally
    pub use_shared_accounts: Option<bool>,
    /// This is useful when the instruction before the swap has a transfer that increases the input token amount.
    /// Then, the swap will just use the difference between the token ledger token amount and post token amount.
    ///
    /// Default: false
    pub use_token_ledger: bool,
    /// Skip RPC calls and assume the user account do not exist,
    /// as a result all setup instruction will be populated but no RPC call will be done for user related accounts (token accounts, openbook open orders...)
    pub skip_user_accounts_rpc_calls: bool,
    /// Providing keyed ui accounts allow loading AMMs that are not in the market cache
    /// If a keyed ui account is the AMM state, it has to be provided with its params according to the market cache format
    pub keyed_ui_accounts: Option<Vec<KeyedUiAccount>>,
    /// The program authority ID
    pub program_authority_id: Option<u8>,
    /// Dynamic slippage
    pub dynamic_slippage: Option<DynamicSlippageSettings>,
    /// Slots to expiry of the blockhash
    pub blockhash_slots_to_expiry: Option<u8>,
    /// Requests a correct last valid block height,
    /// this is to allow a smooth transition to agave 2.0 for all consumers, see https://github.com/solana-labs/solana/issues/24526
    pub correct_last_valid_block_height: bool,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            wrap_and_unwrap_sol: true,
            allow_optimized_wrapped_sol_token_account: false,
            fee_account: None,
            destination_token_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: None,
            prioritization_fee_lamports: None,
            as_legacy_transaction: false,
            use_shared_accounts: None,
            use_token_ledger: false,
            dynamic_compute_unit_limit: false,
            skip_user_accounts_rpc_calls: false,
            keyed_ui_accounts: None,
            program_authority_id: None,
            dynamic_slippage: None,
            blockhash_slots_to_expiry: None,
            correct_last_valid_block_height: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct KeyedUiAccount {
    pub pubkey: String,
    #[serde(flatten)]
    pub ui_account: UiAccount,
    /// Additional data an Amm requires, Amm dependent and decoded in the Amm implementation
    pub params: Option<Value>,
}

```
