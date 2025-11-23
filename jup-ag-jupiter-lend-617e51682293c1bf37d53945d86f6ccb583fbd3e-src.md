## `references/earn/withdraw.rs`

```rust
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

#[error_code]
pub enum ErrorCodes {
    #[msg("CPI_TO_LENDING_PROGRAM_FAILED")]
    CpiToLendingProgramFailed,
}

fn get_withdraw_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:withdraw")[0..8]
    vec![183, 18, 70, 156, 148, 109, 161, 34]
}

pub struct WithdrawParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub owner_token_account: AccountInfo<'info>,
    pub recipient_token_account: AccountInfo<'info>,

    // Protocol accounts
    pub lending_admin: AccountInfo<'info>,
    pub lending: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub f_token_mint: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub lending_supply_position_on_liquidity: AccountInfo<'info>,
    pub rate_model: AccountInfo<'info>,
    pub vault: AccountInfo<'info>,
    pub claim_account: AccountInfo<'info>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,

    // Rewards and programs
    pub rewards_rate_model: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,

    // Target lending program
    pub lending_program: UncheckedAccount<'info>,
}

impl<'info> WithdrawParams<'info> {
    pub fn withdraw(&self, assets: u64) -> Result<()> {
        let mut instruction_data = get_withdraw_discriminator();
        instruction_data.extend_from_slice(&assets.to_le_bytes());

        let account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // owner_token_account (mutable) - user's fToken account
            AccountMeta::new(*self.owner_token_account.key, false),
            // recipient_token_account (mutable) - user's underlying token account
            AccountMeta::new(*self.recipient_token_account.key, false),
            // lending_admin (readonly)
            AccountMeta::new_readonly(*self.lending_admin.key, false),
            // lending (mutable)
            AccountMeta::new(*self.lending.key, false),
            // mint (readonly) - underlying token mint
            AccountMeta::new_readonly(*self.mint.key, false),
            // f_token_mint (mutable)
            AccountMeta::new(*self.f_token_mint.key, false),
            // supply_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            // lending_supply_position_on_liquidity (mutable)
            AccountMeta::new(*self.lending_supply_position_on_liquidity.key, false),
            // rate_model (readonly)
            AccountMeta::new_readonly(*self.rate_model.key, false),
            // vault (mutable)
            AccountMeta::new(*self.vault.key, false),
            // claim_account (mutable)
            AccountMeta::new(*self.claim_account.key, false),
            // liquidity (mutable)
            AccountMeta::new(*self.liquidity.key, false),
            // liquidity_program (mutable)
            AccountMeta::new(*self.liquidity_program.key, false),
            // rewards_rate_model (readonly)
            AccountMeta::new_readonly(*self.rewards_rate_model.key, false),
            // token_program
            AccountMeta::new_readonly(*self.token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.lending_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.owner_token_account.clone(),
                self.recipient_token_account.clone(),
                self.lending_admin.clone(),
                self.lending.clone(),
                self.mint.clone(),
                self.f_token_mint.clone(),
                self.supply_token_reserves_liquidity.clone(),
                self.lending_supply_position_on_liquidity.clone(),
                self.rate_model.clone(),
                self.vault.clone(),
                self.claim_account.clone(),
                self.liquidity.clone(),
                self.liquidity_program.clone(),
                self.rewards_rate_model.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| ErrorCodes::CpiToLendingProgramFailed.into())
    }
}

```
---
## `references/earn/deposit.rs`

```rust
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

#[error_code]
pub enum ErrorCodes {
    #[msg("CPI_TO_LENDING_PROGRAM_FAILED")]
    CpiToLendingProgramFailed,
}

fn get_deposit_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:deposit")[0..8]
    vec![242, 35, 198, 137, 82, 225, 242, 182]
}

pub struct DepositParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub depositor_token_account: AccountInfo<'info>,
    pub recipient_token_account: AccountInfo<'info>,

    pub mint: AccountInfo<'info>,

    // Protocol accounts
    pub lending_admin: AccountInfo<'info>,
    pub lending: AccountInfo<'info>,
    pub f_token_mint: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub lending_supply_position_on_liquidity: AccountInfo<'info>,
    pub rate_model: AccountInfo<'info>,
    pub vault: AccountInfo<'info>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,

    // Rewards and programs
    pub rewards_rate_model: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,

    // Target lending program
    pub lending_program: UncheckedAccount<'info>,
}

impl<'info> DepositParams<'info> {
    pub fn deposit(&self, amount: u64) -> Result<()> {
        let mut instruction_data = get_deposit_discriminator();
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        let account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // depositor_token_account (mutable)
            AccountMeta::new(*self.depositor_token_account.key, false),
            // recipient_token_account (mutable)
            AccountMeta::new(*self.recipient_token_account.key, false),
            // mint
            AccountMeta::new_readonly(*self.mint.key, false),
            // lending_admin (readonly)
            AccountMeta::new_readonly(*self.lending_admin.key, false),
            // lending (mutable)
            AccountMeta::new(*self.lending.key, false),
            // f_token_mint (mutable)
            AccountMeta::new(*self.f_token_mint.key, false),
            // supply_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            // lending_supply_position_on_liquidity (mutable)
            AccountMeta::new(*self.lending_supply_position_on_liquidity.key, false),
            // rate_model (readonly)
            AccountMeta::new_readonly(*self.rate_model.key, false),
            // vault (mutable)
            AccountMeta::new(*self.vault.key, false),
            // liquidity (mutable)
            AccountMeta::new(*self.liquidity.key, false),
            // liquidity_program (mutable)
            AccountMeta::new(*self.liquidity_program.key, false),
            // rewards_rate_model (readonly)
            AccountMeta::new_readonly(*self.rewards_rate_model.key, false),
            // token_program
            AccountMeta::new_readonly(*self.token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.lending_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.depositor_token_account.clone(),
                self.recipient_token_account.clone(),
                self.mint.clone(),
                self.lending_admin.clone(),
                self.lending.clone(),
                self.f_token_mint.clone(),
                self.supply_token_reserves_liquidity.clone(),
                self.lending_supply_position_on_liquidity.clone(),
                self.rate_model.clone(),
                self.vault.clone(),
                self.liquidity.clone(),
                self.liquidity_program.clone(),
                self.rewards_rate_model.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| ErrorCodes::CpiToLendingProgramFailed.into())
    }
}

```
---
## `references/borrow/operate.rs`

```rust
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    pubkey::Pubkey,
};

// Error codes for CPI failures
#[error_code]
pub enum VaultsCpiErrorCodes {
    #[msg("CPI to Vaults program failed")]
    CpiToVaultsProgramFailed,
    #[msg("Invalid remaining accounts indices")]
    InvalidRemainingAccountsIndices,
    #[msg("Missing required claim account")]
    MissingClaimAccount,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TransferType {
    Normal = 0,
    Claim = 1,
}

// Function discriminators
fn get_init_position_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:init_position")[0..8]
    vec![197, 20, 10, 1, 97, 160, 177, 91]
}

fn get_operate_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:operate")[0..8]
    vec![217, 106, 208, 99, 116, 151, 42, 135]
}

pub struct InitPositionParams<'info> {
    pub signer: AccountInfo<'info>,
    pub vault_admin: AccountInfo<'info>,
    pub vault_state: AccountInfo<'info>,
    pub position: AccountInfo<'info>,
    pub position_mint: AccountInfo<'info>,
    pub position_token_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub vaults_program: UncheckedAccount<'info>,
}

impl<'info> InitPositionParams<'info> {
    pub fn init_position(&self, vault_id: u16, next_position_id: u32) -> Result<()> {
        let mut instruction_data = get_init_position_discriminator();
        instruction_data.extend_from_slice(&vault_id.to_le_bytes());
        instruction_data.extend_from_slice(&next_position_id.to_le_bytes());

        let account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // vault_admin (mutable)
            AccountMeta::new(*self.vault_admin.key, false),
            // vault_state (mutable)
            AccountMeta::new(*self.vault_state.key, false),
            // position (mutable)
            AccountMeta::new(*self.position.key, false),
            // position_mint (mutable)
            AccountMeta::new(*self.position_mint.key, false),
            // position_token_account (mutable)
            AccountMeta::new(*self.position_token_account.key, false),
            // token_program
            AccountMeta::new_readonly(*self.token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.vaults_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.vault_admin.clone(),
                self.vault_state.clone(),
                self.position.clone(),
                self.position_mint.clone(),
                self.position_token_account.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| VaultsCpiErrorCodes::CpiToVaultsProgramFailed.into())
    }
}

pub struct OperateParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub signer_supply_token_account: AccountInfo<'info>,
    pub signer_borrow_token_account: AccountInfo<'info>,
    pub recipient: AccountInfo<'info>,
    pub recipient_borrow_token_account: AccountInfo<'info>,
    pub recipient_supply_token_account: AccountInfo<'info>,

    // Vault accounts
    pub vault_config: AccountInfo<'info>,
    pub vault_state: AccountInfo<'info>,
    pub supply_token: AccountInfo<'info>,
    pub borrow_token: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,

    // Position accounts
    pub position: AccountInfo<'info>,
    pub position_token_account: AccountInfo<'info>,
    pub current_position_tick: AccountInfo<'info>,
    pub final_position_tick: AccountInfo<'info>,
    pub current_position_tick_id: AccountInfo<'info>,
    pub final_position_tick_id: AccountInfo<'info>,
    pub new_branch: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub borrow_token_reserves_liquidity: AccountInfo<'info>,
    pub vault_supply_position_on_liquidity: AccountInfo<'info>,
    pub vault_borrow_position_on_liquidity: AccountInfo<'info>,
    pub supply_rate_model: AccountInfo<'info>,
    pub borrow_rate_model: AccountInfo<'info>,
    pub vault_supply_token_account: AccountInfo<'info>,
    pub vault_borrow_token_account: AccountInfo<'info>,
    pub supply_token_claim_account: Option<AccountInfo<'info>>,
    pub borrow_token_claim_account: Option<AccountInfo<'info>>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,
    pub oracle_program: AccountInfo<'info>,

    // Programs
    pub supply_token_program: AccountInfo<'info>,
    pub borrow_token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub vaults_program: UncheckedAccount<'info>,
}

impl<'info> OperateParams<'info> {
    pub fn operate(
        &self,
        new_col: i128,
        new_debt: i128,
        transfer_type: Option<TransferType>,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        // Validate remaining accounts indices
        if remaining_accounts_indices.len() != 3 {
            return Err(VaultsCpiErrorCodes::InvalidRemainingAccountsIndices.into());
        }

        let mut instruction_data = get_operate_discriminator();
        instruction_data.extend_from_slice(&new_col.to_le_bytes());
        instruction_data.extend_from_slice(&new_debt.to_le_bytes());

        // Serialize transfer_type
        match transfer_type {
            Some(t) => {
                instruction_data.push(1); // Some
                instruction_data.push(t as u8);
            }
            None => instruction_data.push(0), // None
        }

        // Serialize remaining_accounts_indices
        instruction_data.push(remaining_accounts_indices.len() as u8);
        instruction_data.extend_from_slice(&remaining_accounts_indices);

        let mut account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // signer_supply_token_account (mutable)
            AccountMeta::new(*self.signer_supply_token_account.key, false),
            // signer_borrow_token_account (mutable)
            AccountMeta::new(*self.signer_borrow_token_account.key, false),
            // recipient
            AccountMeta::new_readonly(*self.recipient.key, false),
            // recipient_borrow_token_account (mutable)
            AccountMeta::new(*self.recipient_borrow_token_account.key, false),
            // recipient_supply_token_account (mutable)
            AccountMeta::new(*self.recipient_supply_token_account.key, false),
            // vault_config (mutable)
            AccountMeta::new(*self.vault_config.key, false),
            // vault_state (mutable)
            AccountMeta::new(*self.vault_state.key, false),
            // supply_token
            AccountMeta::new_readonly(*self.supply_token.key, false),
            // borrow_token
            AccountMeta::new_readonly(*self.borrow_token.key, false),
            // oracle
            AccountMeta::new_readonly(*self.oracle.key, false),
            // position (mutable)
            AccountMeta::new(*self.position.key, false),
            // position_token_account
            AccountMeta::new_readonly(*self.position_token_account.key, false),
            // current_position_tick (mutable)
            AccountMeta::new(*self.current_position_tick.key, false),
            // final_position_tick (mutable)
            AccountMeta::new(*self.final_position_tick.key, false),
            // current_position_tick_id (mutable)
            AccountMeta::new(*self.current_position_tick_id.key, false),
            // final_position_tick_id (mutable)
            AccountMeta::new(*self.final_position_tick_id.key, false),
            // new_branch (mutable)
            AccountMeta::new(*self.new_branch.key, false),
            // supply_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            // borrow_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.borrow_token_reserves_liquidity.key, false),
            // vault_supply_position_on_liquidity (mutable)
            AccountMeta::new(*self.vault_supply_position_on_liquidity.key, false),
            // vault_borrow_position_on_liquidity (mutable)
            AccountMeta::new(*self.vault_borrow_position_on_liquidity.key, false),
            // supply_rate_model (mutable)
            AccountMeta::new(*self.supply_rate_model.key, false),
            // borrow_rate_model (mutable)
            AccountMeta::new(*self.borrow_rate_model.key, false),
            // vault_supply_token_account (mutable)
            AccountMeta::new(*self.vault_supply_token_account.key, false),
            // vault_borrow_token_account (mutable)
            AccountMeta::new(*self.vault_borrow_token_account.key, false),
        ];

        // Add optional claim accounts
        if let Some(ref claim_account) = self.supply_token_claim_account {
            account_metas.push(AccountMeta::new(*claim_account.key, false));
        }
        if let Some(ref claim_account) = self.borrow_token_claim_account {
            account_metas.push(AccountMeta::new(*claim_account.key, false));
        }

        // Add remaining required accounts
        account_metas.extend(vec![
            // liquidity (mutable)
            AccountMeta::new(*self.liquidity.key, false),
            // liquidity_program (mutable)
            AccountMeta::new(*self.liquidity_program.key, false),
            // oracle_program
            AccountMeta::new_readonly(*self.oracle_program.key, false),
            // supply_token_program
            AccountMeta::new_readonly(*self.supply_token_program.key, false),
            // borrow_token_program
            AccountMeta::new_readonly(*self.borrow_token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ]);

        // Add remaining accounts (oracle sources, branches, tick arrays)
        for account in &remaining_accounts {
            account_metas.push(AccountMeta::new(*account.key, false));
        }

        let instruction = Instruction {
            program_id: *self.vaults_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        let mut all_accounts = vec![
            self.signer.clone(),
            self.signer_supply_token_account.clone(),
            self.signer_borrow_token_account.clone(),
            self.recipient.clone(),
            self.recipient_borrow_token_account.clone(),
            self.recipient_supply_token_account.clone(),
            self.vault_config.clone(),
            self.vault_state.clone(),
            self.supply_token.clone(),
            self.borrow_token.clone(),
            self.oracle.clone(),
            self.position.clone(),
            self.position_token_account.clone(),
            self.current_position_tick.clone(),
            self.final_position_tick.clone(),
            self.current_position_tick_id.clone(),
            self.final_position_tick_id.clone(),
            self.new_branch.clone(),
            self.supply_token_reserves_liquidity.clone(),
            self.borrow_token_reserves_liquidity.clone(),
            self.vault_supply_position_on_liquidity.clone(),
            self.vault_borrow_position_on_liquidity.clone(),
            self.supply_rate_model.clone(),
            self.borrow_rate_model.clone(),
            self.vault_supply_token_account.clone(),
            self.vault_borrow_token_account.clone(),
        ];

        // Add optional claim accounts
        if let Some(ref claim_account) = self.supply_token_claim_account {
            all_accounts.push(claim_account.clone());
        }

        if let Some(ref claim_account) = self.borrow_token_claim_account {
            all_accounts.push(claim_account.clone());
        }

        all_accounts.extend(vec![
            self.liquidity.clone(),
            self.liquidity_program.clone(),
            self.oracle_program.clone(),
            self.supply_token_program.clone(),
            self.borrow_token_program.clone(),
            self.associated_token_program.clone(),
            self.system_program.clone(),
        ]);

        // Add remaining accounts
        all_accounts.extend(remaining_accounts);

        invoke(&instruction, &all_accounts)
            .map_err(|_| VaultsCpiErrorCodes::CpiToVaultsProgramFailed.into())?;

        Ok(())
    }

    pub fn deposit(
        &self,
        amount: u64,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        self.operate(
            amount as i128,
            0,
            None,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }

    pub fn withdraw(
        &self,
        amount: u64,
        transfer_type: Option<TransferType>,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        let withdraw_amount = if amount == u64::MAX {
            i128::MIN // Max withdraw
        } else {
            -(amount as i128)
        };

        self.operate(
            withdraw_amount,
            0,
            transfer_type,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }

    pub fn borrow(
        &self,
        amount: u64,
        transfer_type: Option<TransferType>,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        self.operate(
            0,
            amount as i128,
            transfer_type,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }

    pub fn payback(
        &self,
        amount: u64,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        let payback_amount = if amount == u64::MAX {
            i128::MIN // Max payback
        } else {
            -(amount as i128)
        };

        self.operate(
            0,
            payback_amount,
            None,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }

    pub fn deposit_and_borrow(
        &self,
        deposit_amount: u64,
        borrow_amount: u64,
        transfer_type: Option<TransferType>,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        self.operate(
            deposit_amount as i128,
            borrow_amount as i128,
            transfer_type,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }

    pub fn payback_and_withdraw(
        &self,
        payback_amount: u64,
        withdraw_amount: u64,
        transfer_type: Option<TransferType>,
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<()> {
        let payback = if payback_amount == u64::MAX {
            i128::MIN
        } else {
            -(payback_amount as i128)
        };

        let withdraw = if withdraw_amount == u64::MAX {
            i128::MIN
        } else {
            -(withdraw_amount as i128)
        };

        self.operate(
            withdraw,
            payback,
            transfer_type,
            remaining_accounts_indices,
            remaining_accounts,
        )
    }
}

```
---
## `target/idl/lending_reward_rate_model.json`

```json
{
  "address": "jup7TthsMgcR9Y3L277b8Eo9uboVSmu1utkuXHNUKar",
  "metadata": {
    "name": "lending_reward_rate_model",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "cancel_queued_rewards",
      "discriminator": [253, 198, 122, 96, 234, 226, 53, 229],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "lending_account",
          "writable": true
        },
        {
          "name": "mint"
        },
        {
          "name": "f_token_mint"
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true
        },
        {
          "name": "lending_program"
        }
      ],
      "args": []
    },
    {
      "name": "init_lending_rewards_admin",
      "discriminator": [202, 36, 47, 209, 3, 201, 173, 94],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "lending_rewards_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  108, 101, 110, 100, 105, 110, 103, 95, 114, 101, 119, 97, 114,
                  100, 115, 95, 97, 100, 109, 105, 110
                ]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "pubkey"
        },
        {
          "name": "lending_program",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_lending_rewards_rate_model",
      "discriminator": [117, 123, 196, 52, 246, 90, 168, 0],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "mint"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  108, 101, 110, 100, 105, 110, 103, 95, 114, 101, 119, 97, 114,
                  100, 115, 95, 114, 97, 116, 101, 95, 109, 111, 100, 101, 108
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "queue_next_rewards",
      "discriminator": [12, 38, 248, 80, 128, 76, 155, 210],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "lending_account",
          "writable": true
        },
        {
          "name": "mint"
        },
        {
          "name": "f_token_mint"
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true
        },
        {
          "name": "lending_program"
        }
      ],
      "args": [
        {
          "name": "reward_amount",
          "type": "u64"
        },
        {
          "name": "duration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "start_rewards",
      "discriminator": [62, 183, 108, 14, 161, 145, 121, 115],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "lending_account",
          "writable": true
        },
        {
          "name": "mint"
        },
        {
          "name": "f_token_mint"
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true
        },
        {
          "name": "lending_program"
        }
      ],
      "args": [
        {
          "name": "reward_amount",
          "type": "u64"
        },
        {
          "name": "duration",
          "type": "u64"
        },
        {
          "name": "start_time",
          "type": "u64"
        },
        {
          "name": "start_tvl",
          "type": "u64"
        }
      ]
    },
    {
      "name": "stop_rewards",
      "discriminator": [39, 231, 201, 99, 230, 105, 100, 76],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "lending_account",
          "writable": true
        },
        {
          "name": "mint"
        },
        {
          "name": "f_token_mint"
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true
        },
        {
          "name": "lending_program"
        }
      ],
      "args": []
    },
    {
      "name": "transition_to_next_rewards",
      "discriminator": [167, 50, 233, 93, 0, 178, 154, 247],
      "accounts": [
        {
          "name": "lending_rewards_admin"
        },
        {
          "name": "lending_account",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending_rewards_rate_model"]
        },
        {
          "name": "f_token_mint"
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "lending_rewards_rate_model",
          "writable": true
        },
        {
          "name": "lending_program"
        }
      ],
      "args": []
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "lending_rewards_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "LendingRewardsAdmin",
      "discriminator": [68, 18, 109, 18, 2, 9, 174, 101]
    },
    {
      "name": "LendingRewardsRateModel",
      "discriminator": [166, 72, 71, 131, 172, 74, 166, 181]
    }
  ],
  "events": [
    {
      "name": "LogCancelQueuedRewards",
      "discriminator": [177, 173, 63, 139, 228, 173, 187, 204]
    },
    {
      "name": "LogQueueNextRewards",
      "discriminator": [50, 129, 214, 126, 39, 205, 209, 116]
    },
    {
      "name": "LogStartRewards",
      "discriminator": [30, 243, 168, 45, 233, 150, 101, 238]
    },
    {
      "name": "LogStopRewards",
      "discriminator": [37, 218, 239, 232, 21, 149, 99, 31]
    },
    {
      "name": "LogTransitionedToNextRewards",
      "discriminator": [177, 232, 239, 222, 224, 61, 9, 101]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidParams",
      "msg": "LENDING_REWARD_RATE_MODEL_INVALID_PARAMS"
    },
    {
      "code": 6001,
      "name": "AlreadyStopped",
      "msg": "LENDING_REWARD_RATE_MODEL_ALREADY_STOPPED"
    },
    {
      "code": 6002,
      "name": "NextRewardsQueued",
      "msg": "LENDING_REWARD_RATE_MODEL_NEXT_REWARDS_QUEUED"
    },
    {
      "code": 6003,
      "name": "NotEnded",
      "msg": "LENDING_REWARD_RATE_MODEL_NOT_ENDED"
    },
    {
      "code": 6004,
      "name": "NoQueuedRewards",
      "msg": "LENDING_REWARD_RATE_MODEL_NO_QUEUED_REWARDS"
    },
    {
      "code": 6005,
      "name": "MustTransitionToNext",
      "msg": "LENDING_REWARD_RATE_MODEL_MUST_TRANSITION_TO_NEXT"
    },
    {
      "code": 6006,
      "name": "NoRewardsStarted",
      "msg": "LENDING_REWARD_RATE_MODEL_NO_REWARDS_STARTED"
    },
    {
      "code": 6007,
      "name": "MaxAuthCountReached",
      "msg": "LENDING_REWARD_RATE_MODEL_MAX_AUTH_COUNT_REACHED"
    },
    {
      "code": 6008,
      "name": "OnlyAuthority",
      "msg": "LENDING_REWARD_RATE_MODEL_ONLY_AUTHORITY"
    },
    {
      "code": 6009,
      "name": "OnlyAuths",
      "msg": "LENDING_REWARD_RATE_MODEL_ONLY_AUTH"
    },
    {
      "code": 6010,
      "name": "CpiToLendingProgramFailed",
      "msg": "LENDING_REWARD_RATE_MODEL_CPI_TO_LENDING_PROGRAM_FAILED"
    },
    {
      "code": 6011,
      "name": "InvalidLendingProgram",
      "msg": "LENDING_REWARD_RATE_MODEL_INVALID_LENDING_PROGRAM"
    },
    {
      "code": 6012,
      "name": "InvalidMint",
      "msg": "LENDING_REWARD_RATE_MODEL_INVALID_MINT"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "LendingRewardsAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "lending_program",
            "type": "pubkey"
          },
          {
            "name": "auths",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LendingRewardsRateModel",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": ["@dev mint address"],
            "type": "pubkey"
          },
          {
            "name": "start_tvl",
            "docs": [
              "@dev tvl below which rewards rate is 0. If current TVL is below this value, triggering `update_rate()` on the fToken",
              "might bring the total TVL above this cut-off."
            ],
            "type": "u64"
          },
          {
            "name": "duration",
            "docs": ["@dev for how long current rewards should run"],
            "type": "u64"
          },
          {
            "name": "start_time",
            "docs": ["@dev when current rewards got started"],
            "type": "u64"
          },
          {
            "name": "yearly_reward",
            "docs": [
              "@dev current annualized reward based on input params (duration, rewardAmount)"
            ],
            "type": "u64"
          },
          {
            "name": "next_duration",
            "docs": ["@dev Duration for the next rewards phase"],
            "type": "u64"
          },
          {
            "name": "next_reward_amount",
            "docs": ["@dev Amount of rewards for the next phase"],
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogCancelQueuedRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogQueueNextRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "reward_amount",
            "type": "u64"
          },
          {
            "name": "duration",
            "type": "u64"
          },
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogStartRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "reward_amount",
            "type": "u64"
          },
          {
            "name": "duration",
            "type": "u64"
          },
          {
            "name": "start_time",
            "type": "u64"
          },
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogStopRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogTransitionedToNextRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "start_time",
            "type": "u64"
          },
          {
            "name": "end_time",
            "type": "u64"
          },
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/merkle_distributor.json`

```json
{
  "address": "jup9FB8aPL62L8SHwhZJnxnV263qQvc9tseGT6AFLn6",
  "metadata": {
    "name": "merkle_distributor",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "approve_root",
      "discriminator": [167, 152, 175, 193, 218, 188, 184, 23],
      "accounts": [
        {
          "name": "approver",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        },
        {
          "name": "approver_role"
        }
      ],
      "args": [
        {
          "name": "merkle_root",
          "type": {
            "array": ["u8", 32]
          }
        },
        {
          "name": "cycle",
          "type": "u32"
        },
        {
          "name": "start_slot",
          "type": "u32"
        },
        {
          "name": "end_slot",
          "type": "u32"
        }
      ]
    },
    {
      "name": "claim",
      "discriminator": [62, 198, 214, 193, 213, 159, 108, 210],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "merkle_distributor",
          "writable": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "mint",
          "relations": ["merkle_distributor"]
        },
        {
          "name": "claim_status",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [99, 108, 97, 105, 109]
              },
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "arg",
                "path": "position_id"
              },
              {
                "kind": "arg",
                "path": "distributor_id"
              }
            ]
          }
        },
        {
          "name": "vault_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "merkle_admin"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "cumulative_amount",
          "type": "u64"
        },
        {
          "name": "position_type",
          "type": "u8"
        },
        {
          "name": "position_id",
          "type": "pubkey"
        },
        {
          "name": "distributor_id",
          "type": "u32"
        },
        {
          "name": "cycle",
          "type": "u32"
        },
        {
          "name": "merkle_proof",
          "type": {
            "vec": {
              "array": ["u8", 32]
            }
          }
        },
        {
          "name": "metadata",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "distribute_rewards",
      "discriminator": [97, 6, 227, 255, 124, 165, 3, 148],
      "accounts": [
        {
          "name": "distributor",
          "writable": true,
          "signer": true,
          "relations": ["merkle_distributor"]
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["merkle_distributor"]
        },
        {
          "name": "distributor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "distributor"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "vault_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "merkle_admin"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "init_distributor",
      "discriminator": [4, 170, 72, 1, 58, 177, 150, 43],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "merkle_admin",
          "writable": true
        },
        {
          "name": "merkle_distributor",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [100, 105, 115, 116, 114, 105, 98, 117, 116, 111, 114]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "account",
                "path": "merkle_admin.next_distributor_id",
                "account": "MerkleAdmin"
              }
            ]
          }
        },
        {
          "name": "mint"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "InitializeParams"
            }
          }
        }
      ]
    },
    {
      "name": "init_merkle_admin",
      "discriminator": [130, 19, 59, 140, 26, 4, 117, 19],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "merkle_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109, 101, 114, 107, 108, 101, 95, 97, 100, 109, 105, 110
                ]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_role",
      "discriminator": [24, 82, 229, 76, 200, 87, 242, 26],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "role_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [114, 111, 108, 101]
              },
              {
                "kind": "arg",
                "path": "address"
              },
              {
                "kind": "arg",
                "path": "role"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "address",
          "type": "pubkey"
        },
        {
          "name": "role",
          "type": "u8"
        }
      ]
    },
    {
      "name": "pause",
      "discriminator": [211, 22, 221, 251, 74, 121, 193, 47],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "propose_root",
      "discriminator": [132, 0, 76, 107, 236, 86, 118, 165],
      "accounts": [
        {
          "name": "proposer",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        },
        {
          "name": "proposer_role"
        }
      ],
      "args": [
        {
          "name": "merkle_root",
          "type": {
            "array": ["u8", 32]
          }
        },
        {
          "name": "cycle",
          "type": "u32"
        },
        {
          "name": "start_slot",
          "type": "u32"
        },
        {
          "name": "end_slot",
          "type": "u32"
        }
      ]
    },
    {
      "name": "set_start_block_of_next_cycle",
      "discriminator": [168, 83, 36, 171, 34, 95, 238, 81],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "start_block_of_next_cycle",
          "type": "u32"
        }
      ]
    },
    {
      "name": "unpause",
      "discriminator": [169, 144, 4, 38, 10, 141, 188, 255],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_distribution_config",
      "discriminator": [162, 95, 24, 240, 144, 247, 117, 22],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "pull_from_distributor",
          "type": "bool"
        },
        {
          "name": "blocks_per_distribution",
          "type": "u32"
        },
        {
          "name": "cycles_per_distribution",
          "type": "u32"
        }
      ]
    },
    {
      "name": "update_rewards_distributor",
      "discriminator": [250, 201, 40, 213, 158, 61, 253, 147],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "merkle_distributor",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "distributor",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_role",
      "discriminator": [36, 223, 162, 98, 168, 209, 75, 151],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "merkle_admin"
        },
        {
          "name": "role_account",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "turn_off",
          "type": "bool"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "ClaimStatus",
      "discriminator": [22, 183, 249, 157, 247, 95, 150, 96]
    },
    {
      "name": "MerkleAdmin",
      "discriminator": [0, 192, 185, 207, 98, 65, 4, 187]
    },
    {
      "name": "MerkleDistributor",
      "discriminator": [77, 119, 139, 70, 84, 247, 12, 26]
    },
    {
      "name": "RoleAccount",
      "discriminator": [142, 236, 135, 197, 214, 3, 244, 226]
    }
  ],
  "events": [
    {
      "name": "LogClaimed",
      "discriminator": [215, 10, 98, 242, 67, 30, 230, 185]
    },
    {
      "name": "LogDistribution",
      "discriminator": [122, 162, 17, 219, 57, 67, 93, 50]
    },
    {
      "name": "LogDistributionConfigUpdated",
      "discriminator": [64, 108, 152, 215, 83, 217, 187, 190]
    },
    {
      "name": "LogInitRole",
      "discriminator": [14, 236, 197, 243, 241, 106, 70, 162]
    },
    {
      "name": "LogRewardsDistributorUpdated",
      "discriminator": [222, 161, 225, 24, 234, 122, 115, 38]
    },
    {
      "name": "LogRootProposed",
      "discriminator": [241, 45, 0, 250, 225, 243, 158, 34]
    },
    {
      "name": "LogRootUpdated",
      "discriminator": [79, 2, 209, 136, 63, 82, 145, 211]
    },
    {
      "name": "LogStartBlockOfNextCycleUpdated",
      "discriminator": [46, 130, 115, 115, 242, 191, 9, 226]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    },
    {
      "name": "LogUpdateRole",
      "discriminator": [138, 23, 252, 139, 73, 226, 226, 166]
    },
    {
      "name": "Paused",
      "discriminator": [172, 248, 5, 253, 49, 255, 255, 232]
    },
    {
      "name": "Unpaused",
      "discriminator": [156, 150, 47, 174, 120, 216, 93, 117]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidParams",
      "msg": "INVALID_PARAMS"
    },
    {
      "code": 6001,
      "name": "Unauthorized",
      "msg": "UNAUTHORIZED"
    },
    {
      "code": 6002,
      "name": "RewardsPaused",
      "msg": "REWARDS_PAUSED"
    },
    {
      "code": 6003,
      "name": "InvalidCycle",
      "msg": "INVALID_CYCLE"
    },
    {
      "code": 6004,
      "name": "InvalidProof",
      "msg": "INVALID_PROOF"
    },
    {
      "code": 6005,
      "name": "NothingToClaim",
      "msg": "NOTHING_TO_CLAIM"
    },
    {
      "code": 6006,
      "name": "MaxAuthCountReached",
      "msg": "MAX_AUTH_COUNT_REACHED"
    },
    {
      "code": 6007,
      "name": "InvalidDistributor",
      "msg": "INVALID_DISTRIBUTOR"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "ClaimStatus",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "recipient",
            "type": "pubkey"
          },
          {
            "name": "position_id",
            "type": "pubkey"
          },
          {
            "name": "position_type",
            "type": "u8"
          },
          {
            "name": "claimed_amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "InitializeParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distribution_in_hours",
            "type": "u64"
          },
          {
            "name": "cycle_in_hours",
            "type": "u64"
          },
          {
            "name": "start_block",
            "type": "u32"
          },
          {
            "name": "pull_from_distributor",
            "type": "bool"
          },
          {
            "name": "vesting_time",
            "type": "u32"
          },
          {
            "name": "vesting_start_time",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "cycle",
            "type": "u32"
          },
          {
            "name": "position_type",
            "type": "u8"
          },
          {
            "name": "position_id",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "u32"
          },
          {
            "name": "block_number",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogDistribution",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "epoch",
            "type": "u32"
          },
          {
            "name": "distributor",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "start_cycle",
            "type": "u32"
          },
          {
            "name": "end_cycle",
            "type": "u32"
          },
          {
            "name": "registration_block",
            "type": "u32"
          },
          {
            "name": "registration_timestamp",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogDistributionConfigUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "pull_from_distributor",
            "type": "bool"
          },
          {
            "name": "blocks_per_distribution",
            "type": "u32"
          },
          {
            "name": "cycles_per_distribution",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogInitRole",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "address",
            "type": "pubkey"
          },
          {
            "name": "role",
            "type": {
              "defined": {
                "name": "RoleType"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogRewardsDistributorUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "distributor",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogRootProposed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "cycle",
            "type": "u32"
          },
          {
            "name": "merkle_root",
            "type": {
              "array": ["u8", 32]
            }
          },
          {
            "name": "timestamp",
            "type": "u32"
          },
          {
            "name": "publish_block",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogRootUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "cycle",
            "type": "u32"
          },
          {
            "name": "merkle_root",
            "type": {
              "array": ["u8", 32]
            }
          },
          {
            "name": "timestamp",
            "type": "u32"
          },
          {
            "name": "publish_block",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogStartBlockOfNextCycleUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "start_block_of_next_cycle",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateRole",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "address",
            "type": "pubkey"
          },
          {
            "name": "role",
            "type": {
              "defined": {
                "name": "RoleType"
              }
            }
          }
        ]
      }
    },
    {
      "name": "MerkleAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "auths",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "next_distributor_id",
            "type": "u32"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "MerkleCycle",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkle_root",
            "type": {
              "array": ["u8", 32]
            }
          },
          {
            "name": "cycle",
            "type": "u32"
          },
          {
            "name": "timestamp",
            "type": "u32"
          },
          {
            "name": "publish_block",
            "type": "u32"
          },
          {
            "name": "start_slot",
            "type": "u32"
          },
          {
            "name": "end_slot",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "MerkleDistributor",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "distributor_id",
            "type": "u32"
          },
          {
            "name": "paused",
            "type": "u8"
          },
          {
            "name": "distributor",
            "type": "pubkey"
          },
          {
            "name": "current_merkle_cycle",
            "type": {
              "defined": {
                "name": "MerkleCycle"
              }
            }
          },
          {
            "name": "pending_merkle_cycle",
            "type": {
              "defined": {
                "name": "MerkleCycle"
              }
            }
          },
          {
            "name": "previous_merkle_root",
            "type": {
              "array": ["u8", 32]
            }
          },
          {
            "name": "cycles_per_distribution",
            "type": "u32"
          },
          {
            "name": "blocks_per_distribution",
            "type": "u32"
          },
          {
            "name": "start_block_of_next_cycle",
            "type": "u32"
          },
          {
            "name": "end_block_of_last_cycle",
            "type": "u32"
          },
          {
            "name": "pull_from_distributor",
            "type": "u8"
          },
          {
            "name": "vesting_time",
            "type": "u32"
          },
          {
            "name": "vesting_start_time",
            "type": "u32"
          },
          {
            "name": "total_rewards_cycles",
            "type": "u32"
          },
          {
            "name": "total_distributions",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "Paused",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "RoleAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "address",
            "type": "pubkey"
          },
          {
            "name": "role",
            "type": "u8"
          },
          {
            "name": "active",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "RoleType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "None"
          },
          {
            "name": "Proposer"
          },
          {
            "name": "Approver"
          }
        ]
      }
    },
    {
      "name": "Unpaused",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "distributor_id",
            "type": "u32"
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/vaults.json`

```json
{
  "address": "jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi",
  "metadata": {
    "name": "vaults",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "get_exchange_prices",
      "discriminator": [237, 128, 83, 152, 52, 21, 231, 86],
      "accounts": [
        {
          "name": "vault_state"
        },
        {
          "name": "vault_config"
        },
        {
          "name": "supply_token_reserves"
        },
        {
          "name": "borrow_token_reserves"
        }
      ],
      "args": []
    },
    {
      "name": "init_branch",
      "discriminator": [162, 91, 57, 23, 228, 93, 111, 21],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "branch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [98, 114, 97, 110, 99, 104]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              },
              {
                "kind": "arg",
                "path": "branch_id"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "branch_id",
          "type": "u32"
        }
      ]
    },
    {
      "name": "init_position",
      "discriminator": [197, 20, 10, 1, 97, 160, 177, 91],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "position",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [112, 111, 115, 105, 116, 105, 111, 110]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              },
              {
                "kind": "arg",
                "path": "next_position_id"
              }
            ]
          }
        },
        {
          "name": "position_mint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112, 111, 115, 105, 116, 105, 111, 110, 95, 109, 105, 110, 116
                ]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              },
              {
                "kind": "arg",
                "path": "next_position_id"
              }
            ]
          }
        },
        {
          "name": "position_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "const",
                "value": [
                  6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206,
                  235, 121, 172, 28, 180, 133, 237, 95, 91, 55, 145, 58, 140,
                  245, 133, 126, 255, 0, 169
                ]
              },
              {
                "kind": "account",
                "path": "position_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "metadata_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [109, 101, 116, 97, 100, 97, 116, 97]
              },
              {
                "kind": "const",
                "value": [
                  11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107,
                  4, 195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182,
                  209, 188, 3, 248, 41, 70
                ]
              },
              {
                "kind": "account",
                "path": "position_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4,
                195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182, 209,
                188, 3, 248, 41, 70
              ]
            }
          }
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "sysvar_instruction",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "metadata_program",
          "address": "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "next_position_id",
          "type": "u32"
        }
      ]
    },
    {
      "name": "init_tick",
      "discriminator": [22, 13, 62, 141, 73, 89, 178, 29],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "tick_data",
          "writable": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "tick",
          "type": "i32"
        }
      ]
    },
    {
      "name": "init_tick_has_debt_array",
      "discriminator": [206, 108, 146, 245, 20, 0, 141, 208],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "tick_has_debt_array",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116, 105, 99, 107, 95, 104, 97, 115, 95, 100, 101, 98, 116
                ]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              },
              {
                "kind": "arg",
                "path": "index"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "index",
          "type": "u8"
        }
      ]
    },
    {
      "name": "init_tick_id_liquidation",
      "discriminator": [56, 110, 121, 169, 152, 241, 86, 183],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "tick_data",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "tick_id_liquidation",
          "writable": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "tick",
          "type": "i32"
        },
        {
          "name": "total_ids",
          "type": "u32"
        }
      ]
    },
    {
      "name": "init_vault_admin",
      "discriminator": [22, 133, 2, 244, 123, 100, 249, 230],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [118, 97, 117, 108, 116, 95, 97, 100, 109, 105, 110]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "liquidity",
          "type": "pubkey"
        },
        {
          "name": "authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_vault_config",
      "discriminator": [41, 194, 69, 254, 196, 246, 226, 195],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_admin",
          "writable": true
        },
        {
          "name": "vault_config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118, 97, 117, 108, 116, 95, 99, 111, 110, 102, 105, 103
                ]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              }
            ]
          }
        },
        {
          "name": "vault_metadata",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118, 97, 117, 108, 116, 95, 109, 101, 116, 97, 100, 97, 116,
                  97
                ]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              }
            ]
          }
        },
        {
          "name": "oracle"
        },
        {
          "name": "supply_token"
        },
        {
          "name": "borrow_token"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "InitVaultConfigParams"
            }
          }
        }
      ]
    },
    {
      "name": "init_vault_state",
      "discriminator": [96, 120, 23, 100, 153, 11, 13, 165],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault_admin",
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "vault_state",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [118, 97, 117, 108, 116, 95, 115, 116, 97, 116, 101]
              },
              {
                "kind": "arg",
                "path": "vault_id"
              }
            ]
          }
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        }
      ]
    },
    {
      "name": "liquidate",
      "discriminator": [223, 179, 226, 125, 48, 46, 39, 74],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "signer_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "borrow_token_program"
              },
              {
                "kind": "account",
                "path": "borrow_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "to"
        },
        {
          "name": "to_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "to"
              },
              {
                "kind": "account",
                "path": "supply_token_program"
              },
              {
                "kind": "account",
                "path": "supply_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "vault_config",
          "docs": [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ]
        },
        {
          "name": "vault_state",
          "writable": true
        },
        {
          "name": "supply_token"
        },
        {
          "name": "borrow_token"
        },
        {
          "name": "oracle"
        },
        {
          "name": "new_branch",
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "vault_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "vault_borrow_position_on_liquidity",
          "writable": true
        },
        {
          "name": "supply_rate_model"
        },
        {
          "name": "borrow_rate_model"
        },
        {
          "name": "supply_token_claim_account",
          "writable": true,
          "optional": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "liquidity_program"
        },
        {
          "name": "vault_supply_token_account",
          "writable": true
        },
        {
          "name": "vault_borrow_token_account",
          "writable": true
        },
        {
          "name": "supply_token_program"
        },
        {
          "name": "borrow_token_program"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "oracle_program"
        }
      ],
      "args": [
        {
          "name": "debt_amt",
          "type": "u64"
        },
        {
          "name": "col_per_unit_debt",
          "type": "u128"
        },
        {
          "name": "absorb",
          "type": "bool"
        },
        {
          "name": "transfer_type",
          "type": {
            "option": {
              "defined": {
                "name": "TransferType"
              }
            }
          }
        },
        {
          "name": "remaining_accounts_indices",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "operate",
      "discriminator": [217, 106, 208, 99, 116, 151, 42, 135],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "signer_supply_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "supply_token_program"
              },
              {
                "kind": "account",
                "path": "supply_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "signer_borrow_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "borrow_token_program"
              },
              {
                "kind": "account",
                "path": "borrow_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient",
          "optional": true
        },
        {
          "name": "recipient_borrow_token_account",
          "writable": true,
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "recipient"
              },
              {
                "kind": "account",
                "path": "borrow_token_program"
              },
              {
                "kind": "account",
                "path": "borrow_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_supply_token_account",
          "writable": true,
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "recipient"
              },
              {
                "kind": "account",
                "path": "supply_token_program"
              },
              {
                "kind": "account",
                "path": "supply_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "vault_config",
          "docs": [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ]
        },
        {
          "name": "vault_state",
          "docs": ["@dev verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token"
        },
        {
          "name": "borrow_token"
        },
        {
          "name": "oracle"
        },
        {
          "name": "position",
          "writable": true
        },
        {
          "name": "position_token_account",
          "docs": ["@dev verification inside instruction logic"]
        },
        {
          "name": "current_position_tick",
          "writable": true
        },
        {
          "name": "final_position_tick",
          "writable": true
        },
        {
          "name": "current_position_tick_id"
        },
        {
          "name": "final_position_tick_id",
          "writable": true
        },
        {
          "name": "new_branch",
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "vault_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "vault_borrow_position_on_liquidity",
          "writable": true
        },
        {
          "name": "supply_rate_model"
        },
        {
          "name": "borrow_rate_model"
        },
        {
          "name": "vault_supply_token_account",
          "writable": true
        },
        {
          "name": "vault_borrow_token_account",
          "writable": true
        },
        {
          "name": "supply_token_claim_account",
          "writable": true,
          "optional": true
        },
        {
          "name": "borrow_token_claim_account",
          "writable": true,
          "optional": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "liquidity_program"
        },
        {
          "name": "oracle_program"
        },
        {
          "name": "supply_token_program"
        },
        {
          "name": "borrow_token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "new_col",
          "type": "i128"
        },
        {
          "name": "new_debt",
          "type": "i128"
        },
        {
          "name": "transfer_type",
          "type": {
            "option": {
              "defined": {
                "name": "TransferType"
              }
            }
          }
        },
        {
          "name": "remaining_accounts_indices",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "rebalance",
      "discriminator": [108, 158, 77, 9, 210, 52, 88, 62],
      "accounts": [
        {
          "name": "rebalancer",
          "writable": true,
          "signer": true,
          "relations": ["vault_config"]
        },
        {
          "name": "rebalancer_supply_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "rebalancer"
              },
              {
                "kind": "account",
                "path": "supply_token_program"
              },
              {
                "kind": "account",
                "path": "supply_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "rebalancer_borrow_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "rebalancer"
              },
              {
                "kind": "account",
                "path": "borrow_token_program"
              },
              {
                "kind": "account",
                "path": "borrow_token"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "vault_config",
          "docs": [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ],
          "writable": true
        },
        {
          "name": "vault_state",
          "docs": ["@dev verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token",
          "relations": ["vault_config"]
        },
        {
          "name": "borrow_token",
          "relations": ["vault_config"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "vault_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "vault_borrow_position_on_liquidity",
          "writable": true
        },
        {
          "name": "supply_rate_model"
        },
        {
          "name": "borrow_rate_model"
        },
        {
          "name": "liquidity"
        },
        {
          "name": "liquidity_program"
        },
        {
          "name": "vault_supply_token_account",
          "writable": true
        },
        {
          "name": "vault_borrow_token_account",
          "writable": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "supply_token_program"
        },
        {
          "name": "borrow_token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        }
      ],
      "args": []
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_borrow_fee",
      "discriminator": [251, 124, 35, 148, 202, 167, 157, 65],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "borrow_fee",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_borrow_rate_magnifier",
      "discriminator": [75, 250, 27, 176, 156, 53, 26, 112],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "borrow_rate_magnifier",
          "type": "i16"
        }
      ]
    },
    {
      "name": "update_collateral_factor",
      "discriminator": [244, 83, 227, 215, 220, 82, 201, 221],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "collateral_factor",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_core_settings",
      "discriminator": [101, 84, 9, 11, 60, 104, 149, 234],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "UpdateCoreSettingsParams"
            }
          }
        }
      ]
    },
    {
      "name": "update_liquidation_max_limit",
      "discriminator": [183, 242, 152, 150, 176, 40, 65, 161],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "liquidation_max_limit",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_liquidation_penalty",
      "discriminator": [21, 168, 167, 206, 98, 206, 69, 32],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "liquidation_penalty",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_liquidation_threshold",
      "discriminator": [53, 185, 87, 243, 138, 11, 79, 28],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "liquidation_threshold",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_lookup_table",
      "discriminator": [221, 59, 30, 246, 106, 223, 137, 55],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_metadata",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "lookup_table",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_oracle",
      "discriminator": [112, 41, 209, 18, 248, 226, 252, 188],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "new_oracle",
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_rebalancer",
      "discriminator": [206, 187, 54, 228, 145, 8, 203, 111],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "new_rebalancer",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_supply_rate_magnifier",
      "discriminator": [175, 59, 117, 196, 211, 170, 22, 12],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "supply_rate_magnifier",
          "type": "i16"
        }
      ]
    },
    {
      "name": "update_withdraw_gap",
      "discriminator": [229, 163, 76, 21, 82, 215, 25, 233],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "vault_admin"
        },
        {
          "name": "vault_state",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "vault_config",
          "docs": ["@dev Verification inside instruction logic"],
          "writable": true
        },
        {
          "name": "supply_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        },
        {
          "name": "borrow_token_reserves_liquidity",
          "docs": ["@dev Verification inside instruction logic"]
        }
      ],
      "args": [
        {
          "name": "vault_id",
          "type": "u16"
        },
        {
          "name": "withdraw_gap",
          "type": "u16"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "Branch",
      "discriminator": [14, 63, 100, 50, 25, 8, 29, 5]
    },
    {
      "name": "Oracle",
      "discriminator": [139, 194, 131, 179, 140, 179, 229, 244]
    },
    {
      "name": "Position",
      "discriminator": [170, 188, 143, 228, 122, 64, 247, 208]
    },
    {
      "name": "Tick",
      "discriminator": [176, 94, 67, 247, 133, 173, 7, 115]
    },
    {
      "name": "TickHasDebtArray",
      "discriminator": [91, 232, 60, 29, 124, 103, 49, 252]
    },
    {
      "name": "TickIdLiquidation",
      "discriminator": [41, 28, 190, 197, 68, 213, 31, 181]
    },
    {
      "name": "TokenReserve",
      "discriminator": [21, 18, 59, 135, 120, 20, 31, 12]
    },
    {
      "name": "UserBorrowPosition",
      "discriminator": [73, 126, 65, 123, 220, 126, 197, 24]
    },
    {
      "name": "UserSupplyPosition",
      "discriminator": [202, 219, 136, 118, 61, 177, 21, 146]
    },
    {
      "name": "VaultAdmin",
      "discriminator": [88, 97, 160, 117, 102, 39, 103, 44]
    },
    {
      "name": "VaultConfig",
      "discriminator": [99, 86, 43, 216, 184, 102, 119, 77]
    },
    {
      "name": "VaultMetadata",
      "discriminator": [248, 177, 244, 93, 67, 19, 117, 57]
    },
    {
      "name": "VaultState",
      "discriminator": [228, 196, 82, 165, 98, 210, 235, 152]
    }
  ],
  "events": [
    {
      "name": "LogAbsorb",
      "discriminator": [177, 119, 143, 137, 184, 63, 197, 215]
    },
    {
      "name": "LogInitBranch",
      "discriminator": [127, 182, 211, 219, 140, 189, 193, 101]
    },
    {
      "name": "LogInitTick",
      "discriminator": [56, 182, 35, 79, 249, 114, 9, 175]
    },
    {
      "name": "LogInitTickHasDebtArray",
      "discriminator": [15, 134, 113, 2, 251, 206, 30, 129]
    },
    {
      "name": "LogInitTickIdLiquidation",
      "discriminator": [172, 64, 170, 238, 39, 153, 185, 225]
    },
    {
      "name": "LogInitVaultConfig",
      "discriminator": [194, 158, 35, 55, 179, 48, 174, 46]
    },
    {
      "name": "LogInitVaultState",
      "discriminator": [140, 108, 65, 38, 128, 26, 194, 28]
    },
    {
      "name": "LogLiquidate",
      "discriminator": [154, 128, 202, 147, 65, 233, 195, 73]
    },
    {
      "name": "LogOperate",
      "discriminator": [180, 8, 81, 71, 19, 132, 173, 8]
    },
    {
      "name": "LogRebalance",
      "discriminator": [90, 67, 219, 41, 181, 118, 132, 9]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    },
    {
      "name": "LogUpdateBorrowFee",
      "discriminator": [33, 134, 42, 66, 16, 167, 119, 196]
    },
    {
      "name": "LogUpdateBorrowRateMagnifier",
      "discriminator": [186, 23, 46, 117, 57, 111, 107, 51]
    },
    {
      "name": "LogUpdateCollateralFactor",
      "discriminator": [142, 89, 0, 231, 164, 164, 230, 82]
    },
    {
      "name": "LogUpdateCoreSettings",
      "discriminator": [233, 65, 32, 7, 230, 115, 122, 197]
    },
    {
      "name": "LogUpdateExchangePrices",
      "discriminator": [190, 194, 69, 204, 30, 86, 181, 163]
    },
    {
      "name": "LogUpdateLiquidationMaxLimit",
      "discriminator": [73, 32, 49, 0, 234, 86, 150, 94]
    },
    {
      "name": "LogUpdateLiquidationPenalty",
      "discriminator": [42, 132, 67, 48, 209, 133, 77, 83]
    },
    {
      "name": "LogUpdateLiquidationThreshold",
      "discriminator": [211, 71, 215, 239, 159, 238, 71, 219]
    },
    {
      "name": "LogUpdateOracle",
      "discriminator": [251, 163, 219, 57, 30, 152, 177, 10]
    },
    {
      "name": "LogUpdateRebalancer",
      "discriminator": [66, 79, 144, 204, 26, 217, 153, 225]
    },
    {
      "name": "LogUpdateSupplyRateMagnifier",
      "discriminator": [198, 113, 184, 213, 239, 18, 253, 56]
    },
    {
      "name": "LogUpdateWithdrawGap",
      "discriminator": [182, 248, 48, 47, 8, 159, 21, 35]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "VaultNextTickNotFound",
      "msg": "VAULT_NEXT_TICK_NOT_FOUND"
    },
    {
      "code": 6001,
      "name": "VaultInvalidPositionMint",
      "msg": "VAULT_INVALID_POSITION_MINT"
    },
    {
      "code": 6002,
      "name": "VaultTickIdLiquidationMismatch",
      "msg": "VAULT_TICK_ID_LIQUIDATION_MISMATCH"
    },
    {
      "code": 6003,
      "name": "VaultInvalidPositionTokenAmount",
      "msg": "VAULT_INVALID_POSITION_TOKEN_AMOUNT"
    },
    {
      "code": 6004,
      "name": "VaultInvalidRemainingAccountsIndices",
      "msg": "VAULT_INVALID_REMAINING_ACCOUNTS_INDICES"
    },
    {
      "code": 6005,
      "name": "VaultTickHasDebtVaultIdMismatch",
      "msg": "VAULT_TICK_HAS_DEBT_VAULT_ID_MISMATCH"
    },
    {
      "code": 6006,
      "name": "VaultBranchVaultIdMismatch",
      "msg": "VAULT_BRANCH_VAULT_ID_MISMATCH"
    },
    {
      "code": 6007,
      "name": "VaultTickVaultIdMismatch",
      "msg": "VAULT_TICK_VAULT_ID_MISMATCH"
    },
    {
      "code": 6008,
      "name": "VaultInvalidDecimals",
      "msg": "VAULT_INVALID_DECIMALS"
    },
    {
      "code": 6009,
      "name": "VaultInvalidOperateAmount",
      "msg": "VAULT_INVALID_OPERATE_AMOUNT"
    },
    {
      "code": 6010,
      "name": "VaultTickIsEmpty",
      "msg": "VAULT_TICK_IS_EMPTY"
    },
    {
      "code": 6011,
      "name": "VaultPositionAboveCF",
      "msg": "VAULT_POSITION_ABOVE_CF"
    },
    {
      "code": 6012,
      "name": "VaultTopTickDoesNotExist",
      "msg": "VAULT_TOP_TICK_DOES_NOT_EXIST"
    },
    {
      "code": 6013,
      "name": "VaultExcessSlippageLiquidation",
      "msg": "VAULT_EXCESS_SLIPPAGE_LIQUIDATION"
    },
    {
      "code": 6014,
      "name": "VaultNotRebalancer",
      "msg": "VAULT_NOT_REBALANCER"
    },
    {
      "code": 6015,
      "name": "VaultTokenNotInitialized",
      "msg": "VAULT_TOKEN_NOT_INITIALIZED"
    },
    {
      "code": 6016,
      "name": "VaultUserCollateralDebtExceed",
      "msg": "VAULT_USER_COLLATERAL_DEBT_EXCEED"
    },
    {
      "code": 6017,
      "name": "VaultExcessCollateralWithdrawal",
      "msg": "VAULT_EXCESS_COLLATERAL_WITHDRAWAL"
    },
    {
      "code": 6018,
      "name": "VaultExcessDebtPayback",
      "msg": "VAULT_EXCESS_DEBT_PAYBACK"
    },
    {
      "code": 6019,
      "name": "VaultWithdrawMoreThanOperateLimit",
      "msg": "VAULT_WITHDRAW_MORE_THAN_OPERATE_LIMIT"
    },
    {
      "code": 6020,
      "name": "VaultInvalidLiquidationAmt",
      "msg": "VAULT_INVALID_LIQUIDATION_AMT"
    },
    {
      "code": 6021,
      "name": "VaultLiquidationResult",
      "msg": "VAULT_LIQUIDATION_RESULT"
    },
    {
      "code": 6022,
      "name": "VaultBranchDebtTooLow",
      "msg": "VAULT_BRANCH_DEBT_TOO_LOW"
    },
    {
      "code": 6023,
      "name": "VaultTickDebtTooLow",
      "msg": "VAULT_TICK_DEBT_TOO_LOW"
    },
    {
      "code": 6024,
      "name": "VaultLiquidityExchangePriceUnexpected",
      "msg": "VAULT_LIQUIDITY_EXCHANGE_PRICE_UNEXPECTED"
    },
    {
      "code": 6025,
      "name": "VaultUserDebtTooLow",
      "msg": "VAULT_USER_DEBT_TOO_LOW"
    },
    {
      "code": 6026,
      "name": "VaultInvalidPaybackOrDeposit",
      "msg": "VAULT_INVALID_PAYBACK_OR_DEPOSIT"
    },
    {
      "code": 6027,
      "name": "VaultInvalidLiquidation",
      "msg": "VAULT_INVALID_LIQUIDATION"
    },
    {
      "code": 6028,
      "name": "VaultNothingToRebalance",
      "msg": "VAULT_NOTHING_TO_REBALANCE"
    },
    {
      "code": 6029,
      "name": "VaultLiquidationReverts",
      "msg": "VAULT_LIQUIDATION_REVERTS"
    },
    {
      "code": 6030,
      "name": "VaultInvalidOraclePrice",
      "msg": "VAULT_INVALID_ORACLE_PRICE"
    },
    {
      "code": 6031,
      "name": "VaultBranchNotFound",
      "msg": "VAULT_BRANCH_NOT_FOUND"
    },
    {
      "code": 6032,
      "name": "VaultTickNotFound",
      "msg": "VAULT_TICK_NOT_FOUND"
    },
    {
      "code": 6033,
      "name": "VaultTickHasDebtNotFound",
      "msg": "VAULT_TICK_HAS_DEBT_NOT_FOUND"
    },
    {
      "code": 6034,
      "name": "VaultTickMismatch",
      "msg": "VAULT_TICK_MISMATCH"
    },
    {
      "code": 6035,
      "name": "VaultInvalidVaultId",
      "msg": "VAULT_INVALID_VAULT_ID"
    },
    {
      "code": 6036,
      "name": "VaultInvalidNextPositionId",
      "msg": "VAULT_INVALID_NEXT_POSITION_ID"
    },
    {
      "code": 6037,
      "name": "VaultInvalidSupplyMint",
      "msg": "VAULT_INVALID_SUPPLY_MINT"
    },
    {
      "code": 6038,
      "name": "VaultInvalidBorrowMint",
      "msg": "VAULT_INVALID_BORROW_MINT"
    },
    {
      "code": 6039,
      "name": "VaultInvalidOracle",
      "msg": "VAULT_INVALID_ORACLE"
    },
    {
      "code": 6040,
      "name": "VaultInvalidTick",
      "msg": "VAULT_INVALID_TICK"
    },
    {
      "code": 6041,
      "name": "VaultInvalidLiquidityProgram",
      "msg": "VAULT_INVALID_LIQUIDITY_PROGRAM"
    },
    {
      "code": 6042,
      "name": "VaultInvalidPositionAuthority",
      "msg": "VAULT_INVALID_POSITION_AUTHORITY"
    },
    {
      "code": 6043,
      "name": "VaultOracleNotValid",
      "msg": "VAULT_ORACLE_NOT_VALID"
    },
    {
      "code": 6044,
      "name": "VaultBranchOwnerNotValid",
      "msg": "VAULT_BRANCH_OWNER_NOT_VALID"
    },
    {
      "code": 6045,
      "name": "VaultTickHasDebtOwnerNotValid",
      "msg": "VAULT_TICK_HAS_DEBT_OWNER_NOT_VALID"
    },
    {
      "code": 6046,
      "name": "VaultTickOwnerNotValid",
      "msg": "VAULT_TICK_DATA_OWNER_NOT_VALID"
    },
    {
      "code": 6047,
      "name": "VaultLiquidateRemainingAccountsTooShort",
      "msg": "VAULT_LIQUIDATE_REMAINING_ACCOUNTS_TOO_SHORT"
    },
    {
      "code": 6048,
      "name": "VaultOperateRemainingAccountsTooShort",
      "msg": "VAULT_OPERATE_REMAINING_ACCOUNTS_TOO_SHORT"
    },
    {
      "code": 6049,
      "name": "VaultInvalidZerothBranch",
      "msg": "VAULT_INVALID_ZEROTH_BRANCH"
    },
    {
      "code": 6050,
      "name": "VaultCpiToLiquidityFailed",
      "msg": "VAULT_CPY_TO_LIQUIDITY_FAILED"
    },
    {
      "code": 6051,
      "name": "VaultCpiToOracleFailed",
      "msg": "VAULT_CPY_TO_ORACLE_FAILED"
    },
    {
      "code": 6052,
      "name": "VaultOnlyAuthority",
      "msg": "VAULT_ONLY_AUTHORITY"
    },
    {
      "code": 6053,
      "name": "VaultNewBranchInvalid",
      "msg": "VAULT_NEW_BRANCH_INVALID"
    },
    {
      "code": 6054,
      "name": "VaultTickHasDebtIndexMismatch",
      "msg": "VAULT_TICK_HAS_DEBT_INDEX_MISMATCH"
    },
    {
      "code": 6055,
      "name": "VaultTickHasDebtOutOfRange",
      "msg": "VAULT_TICK_HAS_DEBT_OUT_OF_RANGE"
    },
    {
      "code": 6056,
      "name": "VaultUserSupplyPositionRequired",
      "msg": "VAULT_USER_SUPPLY_POSITION_REQUIRED"
    },
    {
      "code": 6057,
      "name": "VaultClaimAccountRequired",
      "msg": "VAULT_CLAIM_ACCOUNT_REQUIRED"
    },
    {
      "code": 6058,
      "name": "VaultRecipientWithdrawAccountRequired",
      "msg": "VAULT_RECIPIENT_WITHDRAW_ACCOUNT_REQUIRED"
    },
    {
      "code": 6059,
      "name": "VaultRecipientBorrowAccountRequired",
      "msg": "VAULT_RECIPIENT_BORROW_ACCOUNT_REQUIRED"
    },
    {
      "code": 6060,
      "name": "VaultPositionAboveLiquidationThreshold",
      "msg": "VAULT_POSITION_ABOVE_LIQUIDATION_THRESHOLD"
    },
    {
      "code": 6061,
      "name": "VaultAdminValueAboveLimit",
      "msg": "VAULT_ADMIN_VALUE_ABOVE_LIMIT"
    },
    {
      "code": 6062,
      "name": "VaultAdminOnlyAuths",
      "msg": "VAULT_ADMIN_ONLY_AUTH_ACCOUNTS"
    },
    {
      "code": 6063,
      "name": "VaultAdminAddressZeroNotAllowed",
      "msg": "VAULT_ADMIN_ADDRESS_ZERO_NOT_ALLOWED"
    },
    {
      "code": 6064,
      "name": "VaultAdminVaultIdMismatch",
      "msg": "VAULT_ADMIN_VAULT_ID_MISMATCH"
    },
    {
      "code": 6065,
      "name": "VaultAdminTotalIdsMismatch",
      "msg": "VAULT_ADMIN_TOTAL_IDS_MISMATCH"
    },
    {
      "code": 6066,
      "name": "VaultAdminTickMismatch",
      "msg": "VAULT_ADMIN_TICK_MISMATCH"
    },
    {
      "code": 6067,
      "name": "VaultAdminLiquidityProgramMismatch",
      "msg": "VAULT_ADMIN_LIQUIDITY_PROGRAM_MISMATCH"
    },
    {
      "code": 6068,
      "name": "VaultAdminMaxAuthCountReached",
      "msg": "VAULT_ADMIN_MAX_AUTH_COUNT_REACHED"
    },
    {
      "code": 6069,
      "name": "VaultAdminInvalidParams",
      "msg": "VAULT_ADMIN_INVALID_PARAMS"
    },
    {
      "code": 6070,
      "name": "VaultAdminOnlyAuthority",
      "msg": "VAULT_ADMIN_ONLY_AUTHORITY"
    },
    {
      "code": 6071,
      "name": "VaultAdminOracleProgramMismatch",
      "msg": "VAULT_ADMIN_ORACLE_PROGRAM_MISMATCH"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "Branch",
      "docs": ["Branch data structure"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "branch_id",
            "type": "u32"
          },
          {
            "name": "status",
            "type": "u8"
          },
          {
            "name": "minima_tick",
            "type": "i32"
          },
          {
            "name": "minima_tick_partials",
            "type": "u32"
          },
          {
            "name": "debt_liquidity",
            "type": "u64"
          },
          {
            "name": "debt_factor",
            "type": "u64"
          },
          {
            "name": "connected_branch_id",
            "type": "u32"
          },
          {
            "name": "connected_minima_tick",
            "type": "i32"
          }
        ]
      }
    },
    {
      "name": "InitVaultConfigParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "supply_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "borrow_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "collateral_factor",
            "type": "u16"
          },
          {
            "name": "liquidation_threshold",
            "type": "u16"
          },
          {
            "name": "liquidation_max_limit",
            "type": "u16"
          },
          {
            "name": "withdraw_gap",
            "type": "u16"
          },
          {
            "name": "liquidation_penalty",
            "type": "u16"
          },
          {
            "name": "borrow_fee",
            "type": "u16"
          },
          {
            "name": "rebalancer",
            "type": "pubkey"
          },
          {
            "name": "liquidity_program",
            "type": "pubkey"
          },
          {
            "name": "oracle_program",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogAbsorb",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "col_amount",
            "type": "u64"
          },
          {
            "name": "debt_amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogInitBranch",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "branch",
            "type": "pubkey"
          },
          {
            "name": "branch_id",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "LogInitTick",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tick",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogInitTickHasDebtArray",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tick_has_debt_array",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogInitTickIdLiquidation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tick_id_liquidation",
            "type": "pubkey"
          },
          {
            "name": "tick",
            "type": "i32"
          }
        ]
      }
    },
    {
      "name": "LogInitVaultConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_config",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogInitVaultState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_state",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogLiquidate",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "signer",
            "type": "pubkey"
          },
          {
            "name": "col_amount",
            "type": "u64"
          },
          {
            "name": "debt_amount",
            "type": "u64"
          },
          {
            "name": "to",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogOperate",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "signer",
            "type": "pubkey"
          },
          {
            "name": "nft_id",
            "type": "u32"
          },
          {
            "name": "new_col",
            "type": "i128"
          },
          {
            "name": "new_debt",
            "type": "i128"
          },
          {
            "name": "to",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogRebalance",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "supply_amt",
            "type": "i128"
          },
          {
            "name": "borrow_amt",
            "type": "i128"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateBorrowFee",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "borrow_fee",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateBorrowRateMagnifier",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "borrow_rate_magnifier",
            "type": "i16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateCollateralFactor",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "collateral_factor",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateCoreSettings",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "supply_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "borrow_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "collateral_factor",
            "type": "u16"
          },
          {
            "name": "liquidation_threshold",
            "type": "u16"
          },
          {
            "name": "liquidation_max_limit",
            "type": "u16"
          },
          {
            "name": "withdraw_gap",
            "type": "u16"
          },
          {
            "name": "liquidation_penalty",
            "type": "u16"
          },
          {
            "name": "borrow_fee",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateExchangePrices",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "vault_borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "liquidity_supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "liquidity_borrow_exchange_price",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogUpdateLiquidationMaxLimit",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "liquidation_max_limit",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateLiquidationPenalty",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "liquidation_penalty",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateLiquidationThreshold",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "liquidation_threshold",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateOracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_oracle",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateRebalancer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_rebalancer",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateSupplyRateMagnifier",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "supply_rate_magnifier",
            "type": "i16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateWithdrawGap",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "withdraw_gap",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "Oracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nonce",
            "type": "u16"
          },
          {
            "name": "sources",
            "type": {
              "vec": {
                "defined": {
                  "name": "Sources"
                }
              }
            }
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "Position",
      "docs": ["Position data structure"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "nft_id",
            "type": "u32"
          },
          {
            "name": "position_mint",
            "type": "pubkey"
          },
          {
            "name": "is_supply_only_position",
            "type": "u8"
          },
          {
            "name": "tick",
            "type": "i32"
          },
          {
            "name": "tick_id",
            "type": "u32"
          },
          {
            "name": "supply_amount",
            "type": "u64"
          },
          {
            "name": "dust_debt_amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "SourceType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Pyth"
          },
          {
            "name": "StakePool"
          }
        ]
      }
    },
    {
      "name": "Sources",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "source",
            "type": "pubkey"
          },
          {
            "name": "invert",
            "type": "bool"
          },
          {
            "name": "multiplier",
            "type": "u128"
          },
          {
            "name": "divisor",
            "type": "u128"
          },
          {
            "name": "source_type",
            "type": {
              "defined": {
                "name": "SourceType"
              }
            }
          }
        ]
      }
    },
    {
      "name": "Tick",
      "docs": ["Tick data structure"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "tick",
            "type": "i32"
          },
          {
            "name": "is_liquidated",
            "type": "u8"
          },
          {
            "name": "total_ids",
            "type": "u32"
          },
          {
            "name": "raw_debt",
            "type": "u64"
          },
          {
            "name": "is_fully_liquidated",
            "type": "u8"
          },
          {
            "name": "liquidation_branch_id",
            "type": "u32"
          },
          {
            "name": "debt_factor",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "TickHasDebt",
      "docs": [
        "Tick has debt structure",
        "Each TickHasDebt can track 8 * 256 = 2048 ticks",
        "children_bits has 32 bytes = 256 bits total",
        "Each map within the array covers 256 ticks"
      ],
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "children_bits",
            "type": {
              "array": ["u8", 32]
            }
          }
        ]
      }
    },
    {
      "name": "TickHasDebtArray",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "index",
            "type": "u8"
          },
          {
            "name": "tick_has_debt",
            "docs": [
              "Each array contains 8 TickHasDebt structs",
              "Each TickHasDebt covers 256 ticks",
              "Total: 8 * 256 = 2048 ticks per TickHasDebtArray"
            ],
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "TickHasDebt"
                  }
                },
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "TickIdLiquidation",
      "docs": ["Tick ID liquidation data"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "tick",
            "type": "i32"
          },
          {
            "name": "tick_map",
            "type": "u32"
          },
          {
            "name": "is_fully_liquidated_1",
            "type": "u8"
          },
          {
            "name": "liquidation_branch_id_1",
            "type": "u32"
          },
          {
            "name": "debt_factor_1",
            "type": "u64"
          },
          {
            "name": "is_fully_liquidated_2",
            "type": "u8"
          },
          {
            "name": "liquidation_branch_id_2",
            "type": "u32"
          },
          {
            "name": "debt_factor_2",
            "type": "u64"
          },
          {
            "name": "is_fully_liquidated_3",
            "type": "u8"
          },
          {
            "name": "liquidation_branch_id_3",
            "type": "u32"
          },
          {
            "name": "debt_factor_3",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "TokenReserve",
      "docs": ["Token configuration and exchange prices"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "borrow_rate",
            "type": "u16"
          },
          {
            "name": "fee_on_interest",
            "type": "u16"
          },
          {
            "name": "last_utilization",
            "type": "u16"
          },
          {
            "name": "last_update_timestamp",
            "type": "u64"
          },
          {
            "name": "supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "max_utilization",
            "type": "u16"
          },
          {
            "name": "total_supply_with_interest",
            "type": "u64"
          },
          {
            "name": "total_supply_interest_free",
            "type": "u64"
          },
          {
            "name": "total_borrow_with_interest",
            "type": "u64"
          },
          {
            "name": "total_borrow_interest_free",
            "type": "u64"
          },
          {
            "name": "total_claim_amount",
            "type": "u64"
          },
          {
            "name": "interacting_protocol",
            "type": "pubkey"
          },
          {
            "name": "interacting_timestamp",
            "type": "u64"
          },
          {
            "name": "interacting_balance",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "TransferType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SKIP"
          },
          {
            "name": "DIRECT"
          },
          {
            "name": "CLAIM"
          }
        ]
      }
    },
    {
      "name": "UpdateCoreSettingsParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "supply_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "borrow_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "collateral_factor",
            "type": "u16"
          },
          {
            "name": "liquidation_threshold",
            "type": "u16"
          },
          {
            "name": "liquidation_max_limit",
            "type": "u16"
          },
          {
            "name": "withdraw_gap",
            "type": "u16"
          },
          {
            "name": "liquidation_penalty",
            "type": "u16"
          },
          {
            "name": "borrow_fee",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "UserBorrowPosition",
      "docs": ["User borrow position"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "protocol",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "with_interest",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "debt_ceiling",
            "type": "u64"
          },
          {
            "name": "last_update",
            "type": "u64"
          },
          {
            "name": "expand_pct",
            "type": "u16"
          },
          {
            "name": "expand_duration",
            "type": "u32"
          },
          {
            "name": "base_debt_ceiling",
            "type": "u64"
          },
          {
            "name": "max_debt_ceiling",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "UserSupplyPosition",
      "docs": ["User supply position"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "protocol",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "with_interest",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "withdrawal_limit",
            "type": "u128"
          },
          {
            "name": "last_update",
            "type": "u64"
          },
          {
            "name": "expand_pct",
            "type": "u16"
          },
          {
            "name": "expand_duration",
            "type": "u64"
          },
          {
            "name": "base_withdrawal_limit",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "VaultAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "liquidity_program",
            "type": "pubkey"
          },
          {
            "name": "next_vault_id",
            "type": "u16"
          },
          {
            "name": "auths",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "VaultConfig",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "supply_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "borrow_rate_magnifier",
            "type": "i16"
          },
          {
            "name": "collateral_factor",
            "type": "u16"
          },
          {
            "name": "liquidation_threshold",
            "type": "u16"
          },
          {
            "name": "liquidation_max_limit",
            "type": "u16"
          },
          {
            "name": "withdraw_gap",
            "type": "u16"
          },
          {
            "name": "liquidation_penalty",
            "type": "u16"
          },
          {
            "name": "borrow_fee",
            "type": "u16"
          },
          {
            "name": "oracle",
            "type": "pubkey"
          },
          {
            "name": "rebalancer",
            "type": "pubkey"
          },
          {
            "name": "liquidity_program",
            "type": "pubkey"
          },
          {
            "name": "oracle_program",
            "type": "pubkey"
          },
          {
            "name": "supply_token",
            "type": "pubkey"
          },
          {
            "name": "borrow_token",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "VaultMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "lookup_table",
            "type": "pubkey"
          },
          {
            "name": "supply_mint_decimals",
            "type": "u8"
          },
          {
            "name": "borrow_mint_decimals",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "VaultState",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault_id",
            "type": "u16"
          },
          {
            "name": "branch_liquidated",
            "type": "u8"
          },
          {
            "name": "topmost_tick",
            "type": "i32"
          },
          {
            "name": "current_branch_id",
            "type": "u32"
          },
          {
            "name": "total_branch_id",
            "type": "u32"
          },
          {
            "name": "total_supply",
            "type": "u64"
          },
          {
            "name": "total_borrow",
            "type": "u64"
          },
          {
            "name": "total_positions",
            "type": "u32"
          },
          {
            "name": "absorbed_debt_amount",
            "type": "u128"
          },
          {
            "name": "absorbed_col_amount",
            "type": "u128"
          },
          {
            "name": "absorbed_dust_debt",
            "type": "u64"
          },
          {
            "name": "liquidity_supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "liquidity_borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "vault_supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "vault_borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "next_position_id",
            "type": "u32"
          },
          {
            "name": "last_update_timestamp",
            "type": "u64"
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/flashloan.json`

```json
{
  "address": "jupgfSgfuAXv4B6R2Uxu85Z1qdzgju79s6MfZekN6XS",
  "metadata": {
    "name": "flashloan",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "activate_protocol",
      "discriminator": [230, 235, 188, 19, 120, 91, 11, 94],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "flashloan_borrow",
      "discriminator": [103, 19, 78, 24, 240, 9, 135, 63],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        },
        {
          "name": "signer_borrow_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint"
        },
        {
          "name": "flashloan_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "flashloan_borrow_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "liquidity_program"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "instruction_sysvar",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "flashloan_payback",
      "discriminator": [213, 47, 153, 137, 84, 243, 94, 232],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        },
        {
          "name": "signer_borrow_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint"
        },
        {
          "name": "flashloan_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "flashloan_borrow_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "liquidity_program"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "instruction_sysvar",
          "address": "Sysvar1nstructions1111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "init_flashloan_admin",
      "discriminator": [185, 117, 154, 56, 95, 12, 187, 139],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102, 108, 97, 115, 104, 108, 111, 97, 110, 95, 97, 100, 109,
                  105, 110
                ]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "pubkey"
        },
        {
          "name": "flashloan_fee",
          "type": "u16"
        },
        {
          "name": "liquidity_program",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "pause_protocol",
      "discriminator": [144, 95, 0, 107, 119, 39, 248, 141],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "set_flashloan_fee",
      "discriminator": [120, 248, 221, 70, 84, 216, 0, 149],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "flashloan_fee",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "flashloan_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "FlashloanAdmin",
      "discriminator": [162, 161, 45, 28, 131, 91, 202, 88]
    }
  ],
  "events": [
    {
      "name": "ActivateProtocol",
      "discriminator": [70, 178, 173, 151, 180, 166, 68, 102]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "PauseProtocol",
      "discriminator": [66, 229, 166, 147, 152, 13, 42, 29]
    },
    {
      "name": "SetFlashloanFee",
      "discriminator": [112, 164, 66, 251, 191, 56, 0, 47]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "FlashloanInvalidAuthority",
      "msg": "FLASHLOAN_INVALID_AUTHORITY"
    },
    {
      "code": 6001,
      "name": "FlashloanFeeTooHigh",
      "msg": "FLASHLOAN_FEE_TOO_HIGH"
    },
    {
      "code": 6002,
      "name": "FlashloanInvalidParams",
      "msg": "FLASHLOAN_INVALID_PARAMS"
    },
    {
      "code": 6003,
      "name": "FlashloanAlreadyActive",
      "msg": "FLASHLOAN_ALREADY_ACTIVE"
    },
    {
      "code": 6004,
      "name": "FlashloanAlreadyInactive",
      "msg": "FLASHLOAN_ALREADY_INACTIVE"
    },
    {
      "code": 6005,
      "name": "FlashloanCpiToLiquidityFailed",
      "msg": "FLASHLOAN_CPI_TO_LIQUIDITY_FAILED"
    },
    {
      "code": 6006,
      "name": "FlashloanNotAllowedInThisSlot",
      "msg": "FLASHLOAN_NOT_ALLOWED_IN_THIS_SLOT"
    },
    {
      "code": 6007,
      "name": "FlashloanInvalidInstructionSysvar",
      "msg": "FLASHLOAN_INVALID_INSTRUCTION_SYSVAR"
    },
    {
      "code": 6008,
      "name": "FlashloanInvalidInstructionData",
      "msg": "FLASHLOAN_INVALID_INSTRUCTION_DATA"
    },
    {
      "code": 6009,
      "name": "FlashloanPaybackNotFound",
      "msg": "FLASHLOAN_PAYBACK_NOT_FOUND"
    }
  ],
  "types": [
    {
      "name": "ActivateProtocol",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "FlashloanAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "liquidity_program",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": "bool"
          },
          {
            "name": "flashloan_fee",
            "type": "u16"
          },
          {
            "name": "flashloan_timestamp",
            "type": "u64"
          },
          {
            "name": "is_flashloan_active",
            "type": "bool"
          },
          {
            "name": "active_flashloan_amount",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "PauseProtocol",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "SetFlashloanFee",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "flashloan_fee",
            "type": "u16"
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/oracle.json`

```json
{
  "address": "jupnw4B6Eqs7ft6rxpzYLJZYSnrpRgPcr589n5Kv4oc",
  "metadata": {
    "name": "oracle",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "get_both_exchange_rate",
      "discriminator": [92, 88, 161, 46, 230, 193, 46, 237],
      "accounts": [
        {
          "name": "oracle"
        }
      ],
      "args": [
        {
          "name": "_nonce",
          "type": "u16"
        }
      ]
    },
    {
      "name": "get_exchange_rate",
      "discriminator": [153, 76, 17, 194, 170, 215, 89, 142],
      "accounts": [
        {
          "name": "oracle"
        }
      ],
      "args": [
        {
          "name": "_nonce",
          "type": "u16"
        }
      ],
      "returns": "u128"
    },
    {
      "name": "get_exchange_rate_liquidate",
      "discriminator": [228, 169, 73, 39, 91, 82, 27, 5],
      "accounts": [
        {
          "name": "oracle"
        }
      ],
      "args": [
        {
          "name": "_nonce",
          "type": "u16"
        }
      ],
      "returns": "u128"
    },
    {
      "name": "get_exchange_rate_operate",
      "discriminator": [174, 166, 126, 10, 122, 153, 94, 203],
      "accounts": [
        {
          "name": "oracle"
        }
      ],
      "args": [
        {
          "name": "_nonce",
          "type": "u16"
        }
      ],
      "returns": "u128"
    },
    {
      "name": "init_admin",
      "discriminator": [97, 65, 97, 27, 200, 206, 72, 219],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "oracle_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111, 114, 97, 99, 108, 101, 95, 97, 100, 109, 105, 110
                ]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_oracle_config",
      "discriminator": [77, 144, 180, 246, 217, 15, 118, 92],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "oracle_admin"
        },
        {
          "name": "oracle",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [111, 114, 97, 99, 108, 101]
              },
              {
                "kind": "arg",
                "path": "nonce"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "sources",
          "type": {
            "vec": {
              "defined": {
                "name": "Sources"
              }
            }
          }
        },
        {
          "name": "nonce",
          "type": "u16"
        }
      ]
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "oracle_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "oracle_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "Oracle",
      "discriminator": [139, 194, 131, 179, 140, 179, 229, 244]
    },
    {
      "name": "OracleAdmin",
      "discriminator": [239, 232, 8, 20, 254, 63, 25, 246]
    }
  ],
  "events": [
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "PriceNotValid",
      "msg": "PRICE_NOT_VALID"
    },
    {
      "code": 6001,
      "name": "PriceTooOld",
      "msg": "PRICE_TOO_OLD"
    },
    {
      "code": 6002,
      "name": "RateZero",
      "msg": "RATE_ZERO"
    },
    {
      "code": 6003,
      "name": "InvalidParams",
      "msg": "INVALID_PARAMS"
    },
    {
      "code": 6004,
      "name": "InvalidPythSourceMultiplierAndDivisor",
      "msg": "INVALID_PYTH_SOURCE_MULTIPLIER_AND_DIVISOR"
    },
    {
      "code": 6005,
      "name": "InvalidSource",
      "msg": "INVALID_SOURCE"
    },
    {
      "code": 6006,
      "name": "InvalidSourcesLength",
      "msg": "INVALID_SOURCES_LENGTH"
    },
    {
      "code": 6007,
      "name": "OracleAdminOnlyAuthority",
      "msg": "ORACLE_ADMIN_ONLY_AUTHORITY"
    },
    {
      "code": 6008,
      "name": "OracleAdminOnlyAuth",
      "msg": "ORACLE_ADMIN_ONLY_AUTH"
    },
    {
      "code": 6009,
      "name": "OracleAdminMaxAuthCountReached",
      "msg": "ORACLE_ADMIN_MAX_AUTH_COUNT_REACHED"
    },
    {
      "code": 6010,
      "name": "OracleAdminInvalidParams",
      "msg": "ORACLE_ADMIN_INVALID_PARAMS"
    },
    {
      "code": 6011,
      "name": "OracleNonceMismatch",
      "msg": "ORACLE_NONCE_MISMATCH"
    },
    {
      "code": 6012,
      "name": "PriceConfidenceNotSufficient",
      "msg": "PRICE_CONFIDENCE_NOT_SUFFICIENT"
    },
    {
      "code": 6013,
      "name": "StakePoolNotRefreshed",
      "msg": "STAKE_POOL_NOT_REFRESHED"
    },
    {
      "code": 6014,
      "name": "InvalidPrice",
      "msg": "INVALID_PRICE"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "Oracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nonce",
            "type": "u16"
          },
          {
            "name": "sources",
            "type": {
              "vec": {
                "defined": {
                  "name": "Sources"
                }
              }
            }
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "OracleAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "auths",
            "type": {
              "vec": "pubkey"
            }
          }
        ]
      }
    },
    {
      "name": "SourceType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Pyth"
          },
          {
            "name": "StakePool"
          },
          {
            "name": "MsolPool"
          }
        ]
      }
    },
    {
      "name": "Sources",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "source",
            "type": "pubkey"
          },
          {
            "name": "invert",
            "type": "bool"
          },
          {
            "name": "multiplier",
            "type": "u128"
          },
          {
            "name": "divisor",
            "type": "u128"
          },
          {
            "name": "source_type",
            "type": {
              "defined": {
                "name": "SourceType"
              }
            }
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/lending.json`

```json
{
  "address": "jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9",
  "metadata": {
    "name": "lending",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "deposit",
      "discriminator": [242, 35, 198, 137, 82, 225, 242, 182],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "depositor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "assets",
          "type": "u64"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "deposit_with_min_amount_out",
      "discriminator": [116, 144, 16, 97, 118, 109, 40, 119],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "depositor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "assets",
          "type": "u64"
        },
        {
          "name": "min_amount_out",
          "type": "u64"
        }
      ]
    },
    {
      "name": "init_lending",
      "discriminator": [156, 224, 67, 46, 89, 189, 157, 209],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "lending_admin",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["token_reserves_liquidity"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102, 95, 116, 111, 107, 101, 110, 95, 109, 105, 110, 116
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "metadata_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [109, 101, 116, 97, 100, 97, 116, 97]
              },
              {
                "kind": "const",
                "value": [
                  11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107,
                  4, 195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182,
                  209, 188, 3, 248, 41, 70
                ]
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4,
                195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182, 209,
                188, 3, 248, 41, 70
              ]
            }
          }
        },
        {
          "name": "lending",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [108, 101, 110, 100, 105, 110, 103]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ]
          }
        },
        {
          "name": "token_reserves_liquidity"
        },
        {
          "name": "token_program"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "sysvar_instruction",
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "metadata_program",
          "address": "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "symbol",
          "type": "string"
        },
        {
          "name": "liquidity_program",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_lending_admin",
      "discriminator": [203, 185, 241, 165, 56, 254, 33, 9],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "lending_admin",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  108, 101, 110, 100, 105, 110, 103, 95, 97, 100, 109, 105, 110
                ]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "liquidity_program",
          "type": "pubkey"
        },
        {
          "name": "rebalancer",
          "type": "pubkey"
        },
        {
          "name": "authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "mint",
      "discriminator": [51, 57, 225, 47, 182, 146, 137, 166],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "depositor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "shares",
          "type": "u64"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "mint_with_max_assets",
      "discriminator": [6, 94, 69, 122, 30, 179, 146, 171],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "depositor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "shares",
          "type": "u64"
        },
        {
          "name": "max_assets",
          "type": "u64"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "rebalance",
      "discriminator": [108, 158, 77, 9, 210, 52, 88, 62],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "depositor_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "redeem",
      "discriminator": [184, 12, 86, 149, 70, 196, 97, 225],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "shares",
          "type": "u64"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "redeem_with_min_amount_out",
      "discriminator": [235, 189, 237, 56, 166, 180, 184, 149],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "shares",
          "type": "u64"
        },
        {
          "name": "min_amount_out",
          "type": "u64"
        }
      ]
    },
    {
      "name": "set_rewards_rate_model",
      "discriminator": [174, 231, 116, 203, 8, 58, 143, 203],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "f_token_mint",
          "relations": ["lending"]
        },
        {
          "name": "new_rewards_rate_model"
        },
        {
          "name": "supply_token_reserves_liquidity"
        }
      ],
      "args": [
        {
          "name": "mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "lending_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "lending_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_rate",
      "discriminator": [24, 225, 53, 189, 72, 212, 225, 178],
      "accounts": [
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "supply_token_reserves_liquidity"]
        },
        {
          "name": "f_token_mint",
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity"
        },
        {
          "name": "rewards_rate_model"
        }
      ],
      "args": []
    },
    {
      "name": "update_rebalancer",
      "discriminator": [206, 187, 54, 228, 145, 8, 203, 111],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "lending_admin",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_rebalancer",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "withdraw",
      "discriminator": [183, 18, 70, 156, 148, 109, 161, 34],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "withdraw_with_max_shares_burn",
      "discriminator": [47, 197, 183, 171, 239, 18, 245, 171],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "f_token_mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "signer"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "lending_admin"
        },
        {
          "name": "lending",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["lending", "rewards_rate_model"]
        },
        {
          "name": "f_token_mint",
          "writable": true,
          "relations": ["lending"]
        },
        {
          "name": "supply_token_reserves_liquidity",
          "writable": true
        },
        {
          "name": "lending_supply_position_on_liquidity",
          "writable": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "liquidity_program",
          "writable": true,
          "relations": ["lending_admin"]
        },
        {
          "name": "rewards_rate_model"
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "max_shares_burn",
          "type": "u64"
        }
      ],
      "returns": "u64"
    }
  ],
  "accounts": [
    {
      "name": "Lending",
      "discriminator": [135, 199, 82, 16, 249, 131, 182, 241]
    },
    {
      "name": "LendingAdmin",
      "discriminator": [42, 8, 33, 220, 163, 40, 210, 5]
    },
    {
      "name": "LendingRewardsRateModel",
      "discriminator": [166, 72, 71, 131, 172, 74, 166, 181]
    },
    {
      "name": "TokenReserve",
      "discriminator": [21, 18, 59, 135, 120, 20, 31, 12]
    },
    {
      "name": "UserSupplyPosition",
      "discriminator": [202, 219, 136, 118, 61, 177, 21, 146]
    }
  ],
  "events": [
    {
      "name": "LogDeposit",
      "discriminator": [176, 243, 1, 56, 142, 206, 1, 106]
    },
    {
      "name": "LogRebalance",
      "discriminator": [90, 67, 219, 41, 181, 118, 132, 9]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    },
    {
      "name": "LogUpdateRates",
      "discriminator": [222, 11, 113, 60, 147, 15, 68, 217]
    },
    {
      "name": "LogUpdateRebalancer",
      "discriminator": [66, 79, 144, 204, 26, 217, 153, 225]
    },
    {
      "name": "LogUpdateRewards",
      "discriminator": [37, 13, 111, 186, 47, 245, 162, 121]
    },
    {
      "name": "LogWithdraw",
      "discriminator": [49, 9, 176, 179, 222, 190, 6, 117]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "FTokenDepositInsignificant",
      "msg": "F_TOKEN_DEPOSIT_INSIGNIFICANT"
    },
    {
      "code": 6001,
      "name": "FTokenMinAmountOut",
      "msg": "F_TOKEN_MIN_AMOUNT_OUT"
    },
    {
      "code": 6002,
      "name": "FTokenMaxAmount",
      "msg": "F_TOKEN_MAX_AMOUNT"
    },
    {
      "code": 6003,
      "name": "FTokenInvalidParams",
      "msg": "F_TOKEN_INVALID_PARAMS"
    },
    {
      "code": 6004,
      "name": "FTokenRewardsRateModelAlreadySet",
      "msg": "F_TOKEN_REWARDS_RATE_MODEL_ALREADY_SET"
    },
    {
      "code": 6005,
      "name": "FTokenMaxAuthCountReached",
      "msg": "F_TOKEN_MAX_AUTH_COUNT"
    },
    {
      "code": 6006,
      "name": "FTokenLiquidityExchangePriceUnexpected",
      "msg": "F_TOKEN_LIQUIDITY_EXCHANGE_PRICE_UNEXPECTED"
    },
    {
      "code": 6007,
      "name": "FTokenCpiToLiquidityFailed",
      "msg": "F_TOKEN_CPI_TO_LIQUIDITY_FAILED"
    },
    {
      "code": 6008,
      "name": "FTokenOnlyAuth",
      "msg": "F_TOKEN_ONLY_AUTH"
    },
    {
      "code": 6009,
      "name": "FTokenOnlyAuthority",
      "msg": "F_TOKEN_ONLY_AUTHORITY"
    },
    {
      "code": 6010,
      "name": "FTokenOnlyRebalancer",
      "msg": "F_TOKEN_ONLY_REBALANCER"
    },
    {
      "code": 6011,
      "name": "FTokenUserSupplyPositionRequired",
      "msg": "F_TOKEN_USER_SUPPLY_POSITION_REQUIRED"
    },
    {
      "code": 6012,
      "name": "FTokenLiquidityProgramMismatch",
      "msg": "F_TOKEN_LIQUIDITY_PROGRAM_MISMATCH"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "Lending",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "f_token_mint",
            "type": "pubkey"
          },
          {
            "name": "lending_id",
            "type": "u16"
          },
          {
            "name": "decimals",
            "docs": ["@dev number of decimals for the fToken, same as ASSET"],
            "type": "u8"
          },
          {
            "name": "rewards_rate_model",
            "docs": [
              "@dev To read PDA of rewards rate model to get_rate instruction"
            ],
            "type": "pubkey"
          },
          {
            "name": "liquidity_exchange_price",
            "docs": [
              "@dev exchange price for the underlying asset in the liquidity protocol (without rewards)"
            ],
            "type": "u64"
          },
          {
            "name": "token_exchange_price",
            "docs": [
              "@dev exchange price between fToken and the underlying asset (with rewards)"
            ],
            "type": "u64"
          },
          {
            "name": "last_update_timestamp",
            "docs": [
              "@dev timestamp when exchange prices were updated the last time"
            ],
            "type": "u64"
          },
          {
            "name": "token_reserves_liquidity",
            "type": "pubkey"
          },
          {
            "name": "supply_position_on_liquidity",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LendingAdmin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "liquidity_program",
            "type": "pubkey"
          },
          {
            "name": "rebalancer",
            "type": "pubkey"
          },
          {
            "name": "next_lending_id",
            "type": "u16"
          },
          {
            "name": "auths",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LendingRewardsRateModel",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": ["@dev mint address"],
            "type": "pubkey"
          },
          {
            "name": "start_tvl",
            "docs": [
              "@dev tvl below which rewards rate is 0. If current TVL is below this value, triggering `update_rate()` on the fToken",
              "might bring the total TVL above this cut-off."
            ],
            "type": "u64"
          },
          {
            "name": "duration",
            "docs": ["@dev for how long current rewards should run"],
            "type": "u64"
          },
          {
            "name": "start_time",
            "docs": ["@dev when current rewards got started"],
            "type": "u64"
          },
          {
            "name": "yearly_reward",
            "docs": [
              "@dev current annualized reward based on input params (duration, rewardAmount)"
            ],
            "type": "u64"
          },
          {
            "name": "next_duration",
            "docs": ["@dev Duration for the next rewards phase"],
            "type": "u64"
          },
          {
            "name": "next_reward_amount",
            "docs": ["@dev Amount of rewards for the next phase"],
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogDeposit",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sender",
            "type": "pubkey"
          },
          {
            "name": "receiver",
            "type": "pubkey"
          },
          {
            "name": "assets",
            "type": "u64"
          },
          {
            "name": "shares_minted",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogRebalance",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "assets",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateRates",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_exchange_price",
            "type": "u64"
          },
          {
            "name": "liquidity_exchange_price",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogUpdateRebalancer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_rebalancer",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateRewards",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "rewards_rate_model",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogWithdraw",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sender",
            "type": "pubkey"
          },
          {
            "name": "receiver",
            "type": "pubkey"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "assets",
            "type": "u64"
          },
          {
            "name": "shares_burned",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "TokenReserve",
      "docs": ["Token configuration and exchange prices"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "borrow_rate",
            "type": "u16"
          },
          {
            "name": "fee_on_interest",
            "type": "u16"
          },
          {
            "name": "last_utilization",
            "type": "u16"
          },
          {
            "name": "last_update_timestamp",
            "type": "u64"
          },
          {
            "name": "supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "max_utilization",
            "type": "u16"
          },
          {
            "name": "total_supply_with_interest",
            "type": "u64"
          },
          {
            "name": "total_supply_interest_free",
            "type": "u64"
          },
          {
            "name": "total_borrow_with_interest",
            "type": "u64"
          },
          {
            "name": "total_borrow_interest_free",
            "type": "u64"
          },
          {
            "name": "total_claim_amount",
            "type": "u64"
          },
          {
            "name": "interacting_protocol",
            "type": "pubkey"
          },
          {
            "name": "interacting_timestamp",
            "type": "u64"
          },
          {
            "name": "interacting_balance",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "UserSupplyPosition",
      "docs": ["User supply position"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "protocol",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "with_interest",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "withdrawal_limit",
            "type": "u128"
          },
          {
            "name": "last_update",
            "type": "u64"
          },
          {
            "name": "expand_pct",
            "type": "u16"
          },
          {
            "name": "expand_duration",
            "type": "u64"
          },
          {
            "name": "base_withdrawal_limit",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    }
  ]
}

```
---
## `target/idl/liquidity.json`

```json
{
  "address": "jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC",
  "metadata": {
    "name": "liquidity",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "change_status",
      "discriminator": [236, 145, 131, 228, 227, 17, 192, 255],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "auth_list"
        }
      ],
      "args": [
        {
          "name": "status",
          "type": "bool"
        }
      ]
    },
    {
      "name": "claim",
      "discriminator": [62, 198, 214, 193, 213, 159, 108, 210],
      "accounts": [
        {
          "name": "user",
          "signer": true,
          "relations": ["claim_account"]
        },
        {
          "name": "liquidity"
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["token_reserve", "claim_account"]
        },
        {
          "name": "recipient_token_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "arg",
                "path": "recipient"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "liquidity"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          },
          "relations": ["token_reserve"]
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        }
      ],
      "args": [
        {
          "name": "recipient",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "close_claim_account",
      "discriminator": [241, 146, 203, 216, 58, 222, 91, 118],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true,
          "relations": ["claim_account"]
        },
        {
          "name": "claim_account",
          "writable": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "_mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "collect_revenue",
      "discriminator": [87, 96, 211, 36, 240, 43, 246, 87],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "auth_list"
        },
        {
          "name": "mint",
          "relations": ["token_reserve"]
        },
        {
          "name": "revenue_collector_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "revenue_collector"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "revenue_collector"
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "liquidity"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          },
          "relations": ["token_reserve"]
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "init_claim_account",
      "discriminator": [112, 141, 47, 170, 42, 99, 144, 145],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "claim_account",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [117, 115, 101, 114, 95, 99, 108, 97, 105, 109]
              },
              {
                "kind": "arg",
                "path": "user"
              },
              {
                "kind": "arg",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "mint",
          "type": "pubkey"
        },
        {
          "name": "user",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_liquidity",
      "discriminator": [95, 189, 216, 183, 188, 62, 244, 108],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "liquidity",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [108, 105, 113, 117, 105, 100, 105, 116, 121]
              }
            ]
          }
        },
        {
          "name": "auth_list",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [97, 117, 116, 104, 95, 108, 105, 115, 116]
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "pubkey"
        },
        {
          "name": "revenue_collector",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_new_protocol",
      "discriminator": [193, 147, 5, 32, 138, 135, 213, 158],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "user_supply_position",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117, 115, 101, 114, 95, 115, 117, 112, 112, 108, 121, 95, 112,
                  111, 115, 105, 116, 105, 111, 110
                ]
              },
              {
                "kind": "arg",
                "path": "supply_mint"
              },
              {
                "kind": "arg",
                "path": "protocol"
              }
            ]
          }
        },
        {
          "name": "user_borrow_position",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117, 115, 101, 114, 95, 98, 111, 114, 114, 111, 119, 95, 112,
                  111, 115, 105, 116, 105, 111, 110
                ]
              },
              {
                "kind": "arg",
                "path": "borrow_mint"
              },
              {
                "kind": "arg",
                "path": "protocol"
              }
            ]
          }
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "supply_mint",
          "type": "pubkey"
        },
        {
          "name": "borrow_mint",
          "type": "pubkey"
        },
        {
          "name": "protocol",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "init_token_reserve",
      "discriminator": [228, 235, 65, 129, 159, 15, 6, 84],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "auth_list"
        },
        {
          "name": "mint"
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "liquidity"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "rate_model",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [114, 97, 116, 101, 95, 109, 111, 100, 101, 108]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "token_reserve",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [114, 101, 115, 101, 114, 118, 101]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "operate",
      "discriminator": [217, 106, 208, 99, 116, 151, 42, 135],
      "accounts": [
        {
          "name": "protocol",
          "signer": true,
          "relations": ["user_supply_position", "user_borrow_position"]
        },
        {
          "name": "liquidity"
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "token_reserve",
            "rate_model",
            "borrow_claim_account",
            "withdraw_claim_account"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "liquidity"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          },
          "relations": ["token_reserve"]
        },
        {
          "name": "user_supply_position",
          "writable": true,
          "optional": true
        },
        {
          "name": "user_borrow_position",
          "writable": true,
          "optional": true
        },
        {
          "name": "rate_model"
        },
        {
          "name": "withdraw_to_account",
          "writable": true,
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "arg",
                "path": "withdraw_to"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "borrow_to_account",
          "writable": true,
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "arg",
                "path": "borrow_to"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          }
        },
        {
          "name": "borrow_claim_account",
          "writable": true,
          "optional": true
        },
        {
          "name": "withdraw_claim_account",
          "writable": true,
          "optional": true
        },
        {
          "name": "token_program"
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        }
      ],
      "args": [
        {
          "name": "supply_amount",
          "type": "i128"
        },
        {
          "name": "borrow_amount",
          "type": "i128"
        },
        {
          "name": "withdraw_to",
          "type": "pubkey"
        },
        {
          "name": "borrow_to",
          "type": "pubkey"
        },
        {
          "name": "transfer_type",
          "type": {
            "defined": {
              "name": "TransferType"
            }
          }
        }
      ]
    },
    {
      "name": "pause_user",
      "discriminator": [18, 63, 43, 94, 239, 53, 101, 14],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "user_supply_position",
          "writable": true
        },
        {
          "name": "user_borrow_position",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "protocol",
          "type": "pubkey"
        },
        {
          "name": "supply_mint",
          "type": "pubkey"
        },
        {
          "name": "borrow_mint",
          "type": "pubkey"
        },
        {
          "name": "supply_status",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "borrow_status",
          "type": {
            "option": "u8"
          }
        }
      ]
    },
    {
      "name": "pre_operate",
      "discriminator": [129, 205, 158, 155, 198, 155, 72, 133],
      "accounts": [
        {
          "name": "protocol",
          "signer": true,
          "relations": ["user_supply_position", "user_borrow_position"]
        },
        {
          "name": "liquidity"
        },
        {
          "name": "user_supply_position",
          "optional": true
        },
        {
          "name": "user_borrow_position",
          "optional": true
        },
        {
          "name": "vault",
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "liquidity"
              },
              {
                "kind": "account",
                "path": "token_program"
              },
              {
                "kind": "arg",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142,
                13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216,
                219, 233, 248, 89
              ]
            }
          },
          "relations": ["token_reserve"]
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "associated_token_program",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "token_program"
        }
      ],
      "args": [
        {
          "name": "mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "unpause_user",
      "discriminator": [71, 115, 128, 252, 182, 126, 234, 62],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "user_supply_position",
          "writable": true
        },
        {
          "name": "user_borrow_position",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "protocol",
          "type": "pubkey"
        },
        {
          "name": "supply_mint",
          "type": "pubkey"
        },
        {
          "name": "borrow_mint",
          "type": "pubkey"
        },
        {
          "name": "supply_status",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "borrow_status",
          "type": {
            "option": "u8"
          }
        }
      ]
    },
    {
      "name": "update_authority",
      "discriminator": [32, 46, 64, 28, 149, 75, 243, 88],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "liquidity",
          "writable": true
        },
        {
          "name": "auth_list",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_authority",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_auths",
      "discriminator": [93, 96, 178, 156, 57, 117, 253, 209],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "auth_list",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "auth_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_exchange_price",
      "discriminator": [239, 244, 10, 248, 116, 25, 53, 150],
      "accounts": [
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "rate_model",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "_mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_guardians",
      "discriminator": [43, 62, 250, 138, 141, 117, 132, 97],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "liquidity"
        },
        {
          "name": "auth_list",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "guardian_status",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressBool"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_rate_data_v1",
      "discriminator": [6, 20, 34, 122, 22, 150, 180, 22],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "rate_model",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["rate_model", "token_reserve"]
        },
        {
          "name": "token_reserve",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "rate_data",
          "type": {
            "defined": {
              "name": "RateDataV1Params"
            }
          }
        }
      ]
    },
    {
      "name": "update_rate_data_v2",
      "discriminator": [116, 73, 53, 146, 216, 45, 228, 124],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "rate_model",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["rate_model", "token_reserve"]
        },
        {
          "name": "token_reserve",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "rate_data",
          "type": {
            "defined": {
              "name": "RateDataV2Params"
            }
          }
        }
      ]
    },
    {
      "name": "update_revenue_collector",
      "discriminator": [167, 142, 124, 240, 220, 113, 141, 59],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "liquidity",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "revenue_collector",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "update_token_config",
      "discriminator": [231, 122, 181, 79, 255, 79, 144, 167],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "rate_model",
          "writable": true
        },
        {
          "name": "mint",
          "relations": ["rate_model", "token_reserve"]
        },
        {
          "name": "token_reserve",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "token_config",
          "type": {
            "defined": {
              "name": "TokenConfig"
            }
          }
        }
      ]
    },
    {
      "name": "update_user_borrow_config",
      "discriminator": [100, 176, 201, 174, 247, 2, 54, 168],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "protocol",
          "relations": ["user_borrow_position"]
        },
        {
          "name": "auth_list"
        },
        {
          "name": "rate_model"
        },
        {
          "name": "mint",
          "relations": ["rate_model", "token_reserve", "user_borrow_position"]
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "user_borrow_position",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "user_borrow_config",
          "type": {
            "defined": {
              "name": "UserBorrowConfig"
            }
          }
        }
      ]
    },
    {
      "name": "update_user_class",
      "discriminator": [12, 206, 68, 135, 63, 212, 48, 119],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "user_class",
          "type": {
            "vec": {
              "defined": {
                "name": "AddressU8"
              }
            }
          }
        }
      ]
    },
    {
      "name": "update_user_supply_config",
      "discriminator": [217, 239, 225, 218, 33, 49, 234, 183],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "protocol",
          "relations": ["user_supply_position"]
        },
        {
          "name": "auth_list"
        },
        {
          "name": "rate_model"
        },
        {
          "name": "mint",
          "relations": ["rate_model", "token_reserve", "user_supply_position"]
        },
        {
          "name": "token_reserve",
          "writable": true
        },
        {
          "name": "user_supply_position",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "user_supply_config",
          "type": {
            "defined": {
              "name": "UserSupplyConfig"
            }
          }
        }
      ]
    },
    {
      "name": "update_user_withdrawal_limit",
      "discriminator": [162, 9, 186, 9, 213, 30, 173, 78],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "auth_list"
        },
        {
          "name": "user_supply_position",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "new_limit",
          "type": "u128"
        },
        {
          "name": "protocol",
          "type": "pubkey"
        },
        {
          "name": "mint",
          "type": "pubkey"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "AuthorizationList",
      "discriminator": [19, 157, 117, 43, 236, 167, 251, 69]
    },
    {
      "name": "Liquidity",
      "discriminator": [54, 252, 249, 226, 137, 172, 121, 58]
    },
    {
      "name": "RateModel",
      "discriminator": [94, 3, 203, 219, 107, 137, 4, 162]
    },
    {
      "name": "TokenReserve",
      "discriminator": [21, 18, 59, 135, 120, 20, 31, 12]
    },
    {
      "name": "UserBorrowPosition",
      "discriminator": [73, 126, 65, 123, 220, 126, 197, 24]
    },
    {
      "name": "UserClaim",
      "discriminator": [228, 142, 195, 181, 228, 147, 32, 209]
    },
    {
      "name": "UserSupplyPosition",
      "discriminator": [202, 219, 136, 118, 61, 177, 21, 146]
    }
  ],
  "events": [
    {
      "name": "LogBorrowRateCap",
      "discriminator": [156, 131, 232, 94, 254, 156, 14, 117]
    },
    {
      "name": "LogChangeStatus",
      "discriminator": [89, 77, 37, 172, 141, 31, 74, 42]
    },
    {
      "name": "LogClaim",
      "discriminator": [238, 50, 157, 85, 151, 58, 231, 45]
    },
    {
      "name": "LogCollectRevenue",
      "discriminator": [64, 198, 22, 194, 123, 87, 166, 82]
    },
    {
      "name": "LogOperate",
      "discriminator": [180, 8, 81, 71, 19, 132, 173, 8]
    },
    {
      "name": "LogPauseUser",
      "discriminator": [100, 17, 114, 224, 180, 30, 52, 170]
    },
    {
      "name": "LogUnpauseUser",
      "discriminator": [170, 91, 132, 96, 179, 77, 168, 26]
    },
    {
      "name": "LogUpdateAuthority",
      "discriminator": [150, 152, 157, 143, 6, 135, 193, 101]
    },
    {
      "name": "LogUpdateAuths",
      "discriminator": [88, 80, 109, 48, 111, 203, 76, 251]
    },
    {
      "name": "LogUpdateExchangePrices",
      "discriminator": [190, 194, 69, 204, 30, 86, 181, 163]
    },
    {
      "name": "LogUpdateGuardians",
      "discriminator": [231, 28, 191, 51, 53, 140, 79, 142]
    },
    {
      "name": "LogUpdateRateDataV1",
      "discriminator": [30, 102, 131, 192, 0, 30, 85, 223]
    },
    {
      "name": "LogUpdateRateDataV2",
      "discriminator": [206, 53, 195, 70, 113, 211, 92, 129]
    },
    {
      "name": "LogUpdateRevenueCollector",
      "discriminator": [44, 143, 80, 250, 211, 147, 180, 159]
    },
    {
      "name": "LogUpdateTokenConfigs",
      "discriminator": [24, 205, 191, 130, 47, 40, 233, 218]
    },
    {
      "name": "LogUpdateUserBorrowConfigs",
      "discriminator": [210, 251, 242, 159, 205, 33, 154, 74]
    },
    {
      "name": "LogUpdateUserClass",
      "discriminator": [185, 193, 106, 248, 11, 53, 0, 136]
    },
    {
      "name": "LogUpdateUserSupplyConfigs",
      "discriminator": [142, 160, 21, 90, 87, 88, 18, 51]
    },
    {
      "name": "LogUpdateUserWithdrawalLimit",
      "discriminator": [114, 131, 152, 189, 120, 253, 88, 105]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "UserClassNotPausable",
      "msg": "ADMIN_MODULE_USER_CLASS_NOT_PAUSABLE"
    },
    {
      "code": 6001,
      "name": "UserClassNotFound",
      "msg": "ADMIN_MODULE_USER_CLASS_NOT_FOUND"
    },
    {
      "code": 6002,
      "name": "UserAlreadyPaused",
      "msg": "ADMIN_MODULE_USER_ALREADY_PAUSED"
    },
    {
      "code": 6003,
      "name": "UserAlreadyUnpaused",
      "msg": "ADMIN_MODULE_USER_ALREADY_UNPAUSED"
    },
    {
      "code": 6004,
      "name": "OnlyLiquidityAuthority",
      "msg": "ADMIN_MODULE_ONLY_LIQUIDITY_AUTHORITY"
    },
    {
      "code": 6005,
      "name": "OnlyAuth",
      "msg": "ADMIN_MODULE_ONLY_AUTH"
    },
    {
      "code": 6006,
      "name": "OnlyGuardians",
      "msg": "ADMIN_MODULE_ONLY_GUARDIANS"
    },
    {
      "code": 6007,
      "name": "InvalidParams",
      "msg": "ADMIN_MODULE_INVALID_PARAMS"
    },
    {
      "code": 6008,
      "name": "InvalidConfigOrder",
      "msg": "ADMIN_MODULE_INVALID_CONFIG_ORDER"
    },
    {
      "code": 6009,
      "name": "StatusAlreadySet",
      "msg": "ADMIN_MODULE_STATUS_ALREADY_SET"
    },
    {
      "code": 6010,
      "name": "LimitsCannotBeZero",
      "msg": "ADMIN_MODULE_LIMITS_CAN_NOT_BE_ZERO"
    },
    {
      "code": 6011,
      "name": "MaxAuthCountReached",
      "msg": "ADMIN_MODULE_MAX_AUTH_COUNT"
    },
    {
      "code": 6012,
      "name": "MaxUserClassesReached",
      "msg": "ADMIN_MODULE_MAX_USER_CLASSES"
    },
    {
      "code": 6013,
      "name": "InsufficientBalance",
      "msg": "USER_MODULE_INSUFFICIENT_BALANCE"
    },
    {
      "code": 6014,
      "name": "UserSupplyPositionRequired",
      "msg": "USER_MODULE_USER_SUPPLY_POSITION_REQUIRED"
    },
    {
      "code": 6015,
      "name": "UserBorrowPositionRequired",
      "msg": "USER_MODULE_USER_BORROW_POSITION_REQUIRED"
    },
    {
      "code": 6016,
      "name": "ClaimAccountRequired",
      "msg": "USER_MODULE_CLAIM_ACCOUNT_REQUIRED"
    },
    {
      "code": 6017,
      "name": "WithdrawToAccountRequired",
      "msg": "USER_MODULE_WITHDRAW_TO_ACCOUNT_REQUIRED"
    },
    {
      "code": 6018,
      "name": "BorrowToAccountRequired",
      "msg": "USER_MODULE_BORROW_TO_ACCOUNT_REQUIRED"
    },
    {
      "code": 6019,
      "name": "InvalidClaimAmount",
      "msg": "USER_MODULE_INVALID_CLAIM_AMOUNT"
    },
    {
      "code": 6020,
      "name": "NoAmountToClaim",
      "msg": "USER_MODULE_NO_AMOUNT_TO_CLAIM"
    },
    {
      "code": 6021,
      "name": "AmountNotZero",
      "msg": "USER_MODULE_AMOUNT_NOT_ZERO"
    },
    {
      "code": 6022,
      "name": "ValueOverflow",
      "msg": "USER_MODULE_VALUE_OVERFLOW"
    },
    {
      "code": 6023,
      "name": "InvalidTransferType",
      "msg": "USER_MODULE_INVALID_TRANSFER_TYPE"
    },
    {
      "code": 6024,
      "name": "MintMismatch",
      "msg": "USER_MODULE_MINT_MISMATCH"
    },
    {
      "code": 6025,
      "name": "UserNotDefined",
      "msg": "USER_MODULE_USER_NOT_DEFINED"
    },
    {
      "code": 6026,
      "name": "InvalidUserClaim",
      "msg": "USER_MODULE_INVALID_USER_CLAIM"
    },
    {
      "code": 6027,
      "name": "UserPaused",
      "msg": "USER_MODULE_USER_PAUSED"
    },
    {
      "code": 6028,
      "name": "WithdrawalLimitReached",
      "msg": "USER_MODULE_WITHDRAWAL_LIMIT_REACHED"
    },
    {
      "code": 6029,
      "name": "BorrowLimitReached",
      "msg": "USER_MODULE_BORROW_LIMIT_REACHED"
    },
    {
      "code": 6030,
      "name": "OperateAmountsNearlyZero",
      "msg": "USER_MODULE_OPERATE_AMOUNTS_ZERO"
    },
    {
      "code": 6031,
      "name": "OperateAmountTooBig",
      "msg": "USER_MODULE_OPERATE_AMOUNTS_TOO_BIG"
    },
    {
      "code": 6032,
      "name": "OperateAmountsInsufficient",
      "msg": "USER_MODULE_OPERATE_AMOUNTS_INSUFFICIENT"
    },
    {
      "code": 6033,
      "name": "TransferAmountOutOfBounds",
      "msg": "USER_MODULE_TRANSFER_AMOUNT_OUT_OF_BOUNDS"
    },
    {
      "code": 6034,
      "name": "ForbiddenOperateCall",
      "msg": "FORBIDDEN_OPERATE_CALL"
    },
    {
      "code": 6035,
      "name": "MaxUtilizationReached",
      "msg": "USER_MODULE_MAX_UTILIZATION_REACHED"
    },
    {
      "code": 6036,
      "name": "ValueOverflowTotalSupply",
      "msg": "USER_MODULE_VALUE_OVERFLOW_TOTAL_SUPPLY"
    },
    {
      "code": 6037,
      "name": "ValueOverflowTotalBorrow",
      "msg": "USER_MODULE_VALUE_OVERFLOW_TOTAL_BORROW"
    },
    {
      "code": 6038,
      "name": "DepositExpected",
      "msg": "USER_MODULE_DEPOSIT_EXPECTED"
    },
    {
      "code": 6039,
      "name": "ExchangePriceZero",
      "msg": "LIQUIDITY_CALCS_EXCHANGE_PRICE_ZERO"
    },
    {
      "code": 6040,
      "name": "UnsupportedRateVersion",
      "msg": "LIQUIDITY_CALCS_UNSUPPORTED_RATE_VERSION"
    },
    {
      "code": 6041,
      "name": "BorrowRateNegative",
      "msg": "LIQUIDITY_CALCS_BORROW_RATE_NEGATIVE"
    },
    {
      "code": 6042,
      "name": "ProtocolLockdown",
      "msg": "PROTOCOL_LOCKDOWN"
    }
  ],
  "types": [
    {
      "name": "AddressBool",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "AddressU8",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "value",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "AuthorizationList",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_users",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "guardians",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "user_classes",
            "type": {
              "vec": {
                "defined": {
                  "name": "UserClass"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "Liquidity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "revenue_collector",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogBorrowRateCap",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogChangeStatus",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_status",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "LogClaim",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "recipient",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogCollectRevenue",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "revenue_amount",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "LogOperate",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "supply_amount",
            "type": "i128"
          },
          {
            "name": "borrow_amount",
            "type": "i128"
          },
          {
            "name": "withdraw_to",
            "type": "pubkey"
          },
          {
            "name": "borrow_to",
            "type": "pubkey"
          },
          {
            "name": "supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "borrow_exchange_price",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LogPauseUser",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogUnpauseUser",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "new_authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateAuths",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "auth_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateExchangePrices",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "supply_exchange_price",
            "type": "u128"
          },
          {
            "name": "borrow_exchange_price",
            "type": "u128"
          },
          {
            "name": "borrow_rate",
            "type": "u16"
          },
          {
            "name": "utilization",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "LogUpdateGuardians",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "guardian_status",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressBool"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateRateDataV1",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "rate_data",
            "type": {
              "defined": {
                "name": "RateDataV1Params"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateRateDataV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "rate_data",
            "type": {
              "defined": {
                "name": "RateDataV2Params"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateRevenueCollector",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "revenue_collector",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "LogUpdateTokenConfigs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_config",
            "type": {
              "defined": {
                "name": "TokenConfig"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateUserBorrowConfigs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "user_borrow_config",
            "type": {
              "defined": {
                "name": "UserBorrowConfig"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateUserClass",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user_class",
            "type": {
              "vec": {
                "defined": {
                  "name": "AddressU8"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateUserSupplyConfigs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "user_supply_config",
            "type": {
              "defined": {
                "name": "UserSupplyConfig"
              }
            }
          }
        ]
      }
    },
    {
      "name": "LogUpdateUserWithdrawalLimit",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": "pubkey"
          },
          {
            "name": "new_limit",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "RateDataV1Params",
      "docs": ["@notice struct to set borrow rate data for version 1"],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "kink",
            "docs": [
              "",
              "@param kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_zero",
            "docs": [
              "",
              "@param rateAtUtilizationZero desired borrow rate when utilization is zero. in 1e2: 100% = 10_000; 1% = 100",
              "i.e. constant minimum borrow rate",
              "e.g. at utilization = 0.01% rate could still be at least 4% (rateAtUtilizationZero would be 400 then)"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_kink",
            "docs": [
              "",
              "@param rateAtUtilizationKink borrow rate when utilization is at kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at kink then rateAtUtilizationKink would be 700"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_max",
            "docs": [
              "",
              "@param rateAtUtilizationMax borrow rate when utilization is maximum at 100%. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 125% at 100% then rateAtUtilizationMax would be 12_500"
            ],
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "RateDataV2Params",
      "docs": ["@notice struct to set borrow rate data for version 2"],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "kink1",
            "docs": [
              "",
              "@param kink1 first kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100",
              "utilization below kink 1 usually means slow increase in rate, once utilization is above kink 1 borrow rate increases faster"
            ],
            "type": "u128"
          },
          {
            "name": "kink2",
            "docs": [
              "",
              "@param kink2 second kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100",
              "utilization below kink 2 usually means slow / medium increase in rate, once utilization is above kink 2 borrow rate increases fast"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_zero",
            "docs": [
              "",
              "@param rateAtUtilizationZero desired borrow rate when utilization is zero. in 1e2: 100% = 10_000; 1% = 100",
              "i.e. constant minimum borrow rate",
              "e.g. at utilization = 0.01% rate could still be at least 4% (rateAtUtilizationZero would be 400 then)"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_kink1",
            "docs": [
              "",
              "@param rateAtUtilizationKink1 desired borrow rate when utilization is at first kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at first kink then rateAtUtilizationKink would be 700"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_kink2",
            "docs": [
              "",
              "@param rateAtUtilizationKink2 desired borrow rate when utilization is at second kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at second kink then rateAtUtilizationKink would be 1_200"
            ],
            "type": "u128"
          },
          {
            "name": "rate_at_utilization_max",
            "docs": [
              "",
              "@param rateAtUtilizationMax desired borrow rate when utilization is maximum at 100%. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 125% at 100% then rateAtUtilizationMax would be 12_500"
            ],
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "RateModel",
      "docs": ["Interest rate model data"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "rate_at_zero",
            "type": "u16"
          },
          {
            "name": "kink1_utilization",
            "type": "u16"
          },
          {
            "name": "rate_at_kink1",
            "type": "u16"
          },
          {
            "name": "rate_at_max",
            "type": "u16"
          },
          {
            "name": "kink2_utilization",
            "type": "u16"
          },
          {
            "name": "rate_at_kink2",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "TokenConfig",
      "docs": ["@notice struct to set token config"],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "docs": ["", "@param token address"],
            "type": "pubkey"
          },
          {
            "name": "fee",
            "docs": [
              "",
              "@param fee charges on borrower's interest. in 1e2: 100% = 10_000; 1% = 100"
            ],
            "type": "u128"
          },
          {
            "name": "max_utilization",
            "docs": [
              "",
              "@param maxUtilization maximum allowed utilization. in 1e2: 100% = 10_000; 1% = 100",
              "set to 100% to disable and have default limit of 100% (avoiding SLOAD)."
            ],
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "TokenReserve",
      "docs": ["Token configuration and exchange prices"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "borrow_rate",
            "type": "u16"
          },
          {
            "name": "fee_on_interest",
            "type": "u16"
          },
          {
            "name": "last_utilization",
            "type": "u16"
          },
          {
            "name": "last_update_timestamp",
            "type": "u64"
          },
          {
            "name": "supply_exchange_price",
            "type": "u64"
          },
          {
            "name": "borrow_exchange_price",
            "type": "u64"
          },
          {
            "name": "max_utilization",
            "type": "u16"
          },
          {
            "name": "total_supply_with_interest",
            "type": "u64"
          },
          {
            "name": "total_supply_interest_free",
            "type": "u64"
          },
          {
            "name": "total_borrow_with_interest",
            "type": "u64"
          },
          {
            "name": "total_borrow_interest_free",
            "type": "u64"
          },
          {
            "name": "total_claim_amount",
            "type": "u64"
          },
          {
            "name": "interacting_protocol",
            "type": "pubkey"
          },
          {
            "name": "interacting_timestamp",
            "type": "u64"
          },
          {
            "name": "interacting_balance",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "TransferType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SKIP"
          },
          {
            "name": "DIRECT"
          },
          {
            "name": "CLAIM"
          }
        ]
      }
    },
    {
      "name": "UserBorrowConfig",
      "docs": ["@notice struct to set user borrow & payback config"],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mode",
            "docs": [
              "",
              "@param mode: 0 = without interest. 1 = with interest"
            ],
            "type": "u8"
          },
          {
            "name": "expand_percent",
            "docs": [
              "",
              "@param expandPercent debt limit expand percent. in 1e2: 100% = 10_000; 1% = 100",
              "Also used to calculate rate at which debt limit should decrease (instant)."
            ],
            "type": "u128"
          },
          {
            "name": "expand_duration",
            "docs": [
              "",
              "@param expandDuration debt limit expand duration in seconds.",
              "used to calculate rate together with expandPercent"
            ],
            "type": "u128"
          },
          {
            "name": "base_debt_ceiling",
            "docs": [
              "",
              "@param baseDebtCeiling base borrow limit. until here, borrow limit remains as baseDebtCeiling",
              "(user can borrow until this point at once without stepped expansion). Above this, automated limit comes in place.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ],
            "type": "u128"
          },
          {
            "name": "max_debt_ceiling",
            "docs": [
              "",
              "@param maxDebtCeiling max borrow ceiling, maximum amount the user can borrow.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ],
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "UserBorrowPosition",
      "docs": ["User borrow position"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "protocol",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "with_interest",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "debt_ceiling",
            "type": "u64"
          },
          {
            "name": "last_update",
            "type": "u64"
          },
          {
            "name": "expand_pct",
            "type": "u16"
          },
          {
            "name": "expand_duration",
            "type": "u32"
          },
          {
            "name": "base_debt_ceiling",
            "type": "u64"
          },
          {
            "name": "max_debt_ceiling",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "UserClaim",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "UserClass",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addr",
            "type": "pubkey"
          },
          {
            "name": "class",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "UserSupplyConfig",
      "docs": ["@notice struct to set user supply & withdrawal config"],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mode",
            "docs": [
              "",
              "@param mode: 0 = without interest. 1 = with interest"
            ],
            "type": "u8"
          },
          {
            "name": "expand_percent",
            "docs": [
              "",
              "@param expandPercent withdrawal limit expand percent. in 1e2: 100% = 10_000; 1% = 100",
              "Also used to calculate rate at which withdrawal limit should decrease (instant)."
            ],
            "type": "u128"
          },
          {
            "name": "expand_duration",
            "docs": [
              "",
              "@param expandDuration withdrawal limit expand duration in seconds.",
              "used to calculate rate together with expandPercent"
            ],
            "type": "u128"
          },
          {
            "name": "base_withdrawal_limit",
            "docs": [
              "",
              "@param baseWithdrawalLimit base limit, below this, user can withdraw the entire amount.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ],
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "UserSupplyPosition",
      "docs": ["User supply position"],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c",
        "packed": true
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "protocol",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "with_interest",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "withdrawal_limit",
            "type": "u128"
          },
          {
            "name": "last_update",
            "type": "u64"
          },
          {
            "name": "expand_pct",
            "type": "u16"
          },
          {
            "name": "expand_duration",
            "type": "u64"
          },
          {
            "name": "base_withdrawal_limit",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          }
        ]
      }
    }
  ]
}

```
---
## `target/types/lending_reward_rate_model.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/lending_reward_rate_model.json`.
 */
export type LendingRewardRateModel = {
  address: "jup7TthsMgcR9Y3L277b8Eo9uboVSmu1utkuXHNUKar";
  metadata: {
    name: "lendingRewardRateModel";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "cancelQueuedRewards";
      discriminator: [253, 198, 122, 96, 234, 226, 53, 229];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "lendingAccount";
          writable: true;
        },
        {
          name: "mint";
        },
        {
          name: "fTokenMint";
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
        },
        {
          name: "lendingProgram";
        }
      ];
      args: [];
    },
    {
      name: "initLendingRewardsAdmin";
      discriminator: [202, 36, 47, 209, 3, 201, 173, 94];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  108,
                  101,
                  110,
                  100,
                  105,
                  110,
                  103,
                  95,
                  114,
                  101,
                  119,
                  97,
                  114,
                  100,
                  115,
                  95,
                  97,
                  100,
                  109,
                  105,
                  110
                ];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "authority";
          type: "pubkey";
        },
        {
          name: "lendingProgram";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initLendingRewardsRateModel";
      discriminator: [117, 123, 196, 52, 246, 90, 168, 0];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "mint";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  108,
                  101,
                  110,
                  100,
                  105,
                  110,
                  103,
                  95,
                  114,
                  101,
                  119,
                  97,
                  114,
                  100,
                  115,
                  95,
                  114,
                  97,
                  116,
                  101,
                  95,
                  109,
                  111,
                  100,
                  101,
                  108
                ];
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [];
    },
    {
      name: "queueNextRewards";
      discriminator: [12, 38, 248, 80, 128, 76, 155, 210];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "lendingAccount";
          writable: true;
        },
        {
          name: "mint";
        },
        {
          name: "fTokenMint";
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
        },
        {
          name: "lendingProgram";
        }
      ];
      args: [
        {
          name: "rewardAmount";
          type: "u64";
        },
        {
          name: "duration";
          type: "u64";
        }
      ];
    },
    {
      name: "startRewards";
      discriminator: [62, 183, 108, 14, 161, 145, 121, 115];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "lendingAccount";
          writable: true;
        },
        {
          name: "mint";
        },
        {
          name: "fTokenMint";
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
        },
        {
          name: "lendingProgram";
        }
      ];
      args: [
        {
          name: "rewardAmount";
          type: "u64";
        },
        {
          name: "duration";
          type: "u64";
        },
        {
          name: "startTime";
          type: "u64";
        },
        {
          name: "startTvl";
          type: "u64";
        }
      ];
    },
    {
      name: "stopRewards";
      discriminator: [39, 231, 201, 99, 230, 105, 100, 76];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "lendingAccount";
          writable: true;
        },
        {
          name: "mint";
        },
        {
          name: "fTokenMint";
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
        },
        {
          name: "lendingProgram";
        }
      ];
      args: [];
    },
    {
      name: "transitionToNextRewards";
      discriminator: [167, 50, 233, 93, 0, 178, 154, 247];
      accounts: [
        {
          name: "lendingRewardsAdmin";
        },
        {
          name: "lendingAccount";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lendingRewardsRateModel"];
        },
        {
          name: "fTokenMint";
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "lendingRewardsRateModel";
          writable: true;
        },
        {
          name: "lendingProgram";
        }
      ];
      args: [];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "lendingRewardsAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    }
  ];
  accounts: [
    {
      name: "lendingRewardsAdmin";
      discriminator: [68, 18, 109, 18, 2, 9, 174, 101];
    },
    {
      name: "lendingRewardsRateModel";
      discriminator: [166, 72, 71, 131, 172, 74, 166, 181];
    }
  ];
  events: [
    {
      name: "logCancelQueuedRewards";
      discriminator: [177, 173, 63, 139, 228, 173, 187, 204];
    },
    {
      name: "logQueueNextRewards";
      discriminator: [50, 129, 214, 126, 39, 205, 209, 116];
    },
    {
      name: "logStartRewards";
      discriminator: [30, 243, 168, 45, 233, 150, 101, 238];
    },
    {
      name: "logStopRewards";
      discriminator: [37, 218, 239, 232, 21, 149, 99, 31];
    },
    {
      name: "logTransitionedToNextRewards";
      discriminator: [177, 232, 239, 222, 224, 61, 9, 101];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "invalidParams";
      msg: "lendingRewardRateModelInvalidParams";
    },
    {
      code: 6001;
      name: "alreadyStopped";
      msg: "lendingRewardRateModelAlreadyStopped";
    },
    {
      code: 6002;
      name: "nextRewardsQueued";
      msg: "lendingRewardRateModelNextRewardsQueued";
    },
    {
      code: 6003;
      name: "notEnded";
      msg: "lendingRewardRateModelNotEnded";
    },
    {
      code: 6004;
      name: "noQueuedRewards";
      msg: "lendingRewardRateModelNoQueuedRewards";
    },
    {
      code: 6005;
      name: "mustTransitionToNext";
      msg: "lendingRewardRateModelMustTransitionToNext";
    },
    {
      code: 6006;
      name: "noRewardsStarted";
      msg: "lendingRewardRateModelNoRewardsStarted";
    },
    {
      code: 6007;
      name: "maxAuthCountReached";
      msg: "lendingRewardRateModelMaxAuthCountReached";
    },
    {
      code: 6008;
      name: "onlyAuthority";
      msg: "lendingRewardRateModelOnlyAuthority";
    },
    {
      code: 6009;
      name: "onlyAuths";
      msg: "lendingRewardRateModelOnlyAuth";
    },
    {
      code: 6010;
      name: "cpiToLendingProgramFailed";
      msg: "lendingRewardRateModelCpiToLendingProgramFailed";
    },
    {
      code: 6011;
      name: "invalidLendingProgram";
      msg: "lendingRewardRateModelInvalidLendingProgram";
    },
    {
      code: 6012;
      name: "invalidMint";
      msg: "lendingRewardRateModelInvalidMint";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "lendingRewardsAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "lendingProgram";
            type: "pubkey";
          },
          {
            name: "auths";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "lendingRewardsRateModel";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            docs: ["@dev mint address"];
            type: "pubkey";
          },
          {
            name: "startTvl";
            docs: [
              "@dev tvl below which rewards rate is 0. If current TVL is below this value, triggering `update_rate()` on the fToken",
              "might bring the total TVL above this cut-off."
            ];
            type: "u64";
          },
          {
            name: "duration";
            docs: ["@dev for how long current rewards should run"];
            type: "u64";
          },
          {
            name: "startTime";
            docs: ["@dev when current rewards got started"];
            type: "u64";
          },
          {
            name: "yearlyReward";
            docs: [
              "@dev current annualized reward based on input params (duration, rewardAmount)"
            ];
            type: "u64";
          },
          {
            name: "nextDuration";
            docs: ["@dev Duration for the next rewards phase"];
            type: "u64";
          },
          {
            name: "nextRewardAmount";
            docs: ["@dev Amount of rewards for the next phase"];
            type: "u64";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logCancelQueuedRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logQueueNextRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "rewardAmount";
            type: "u64";
          },
          {
            name: "duration";
            type: "u64";
          },
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logStartRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "rewardAmount";
            type: "u64";
          },
          {
            name: "duration";
            type: "u64";
          },
          {
            name: "startTime";
            type: "u64";
          },
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logStopRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logTransitionedToNextRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "startTime";
            type: "u64";
          },
          {
            name: "endTime";
            type: "u64";
          },
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/lending.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/lending.json`.
 */
export type Lending = {
  address: "jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9";
  metadata: {
    name: "lending";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "deposit";
      discriminator: [242, 35, 198, 137, 82, 225, 242, 182];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "depositorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "assets";
          type: "u64";
        }
      ];
      returns: "u64";
    },
    {
      name: "depositWithMinAmountOut";
      discriminator: [116, 144, 16, 97, 118, 109, 40, 119];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "depositorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "assets";
          type: "u64";
        },
        {
          name: "minAmountOut";
          type: "u64";
        }
      ];
    },
    {
      name: "initLending";
      discriminator: [156, 224, 67, 46, 89, 189, 157, 209];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "lendingAdmin";
          writable: true;
        },
        {
          name: "mint";
          relations: ["tokenReservesLiquidity"];
        },
        {
          name: "fTokenMint";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  102,
                  95,
                  116,
                  111,
                  107,
                  101,
                  110,
                  95,
                  109,
                  105,
                  110,
                  116
                ];
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
          };
        },
        {
          name: "metadataAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [109, 101, 116, 97, 100, 97, 116, 97];
              },
              {
                kind: "const";
                value: [
                  11,
                  112,
                  101,
                  177,
                  227,
                  209,
                  124,
                  69,
                  56,
                  157,
                  82,
                  127,
                  107,
                  4,
                  195,
                  205,
                  88,
                  184,
                  108,
                  115,
                  26,
                  160,
                  253,
                  181,
                  73,
                  182,
                  209,
                  188,
                  3,
                  248,
                  41,
                  70
                ];
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                11,
                112,
                101,
                177,
                227,
                209,
                124,
                69,
                56,
                157,
                82,
                127,
                107,
                4,
                195,
                205,
                88,
                184,
                108,
                115,
                26,
                160,
                253,
                181,
                73,
                182,
                209,
                188,
                3,
                248,
                41,
                70
              ];
            };
          };
        },
        {
          name: "lending";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [108, 101, 110, 100, 105, 110, 103];
              },
              {
                kind: "account";
                path: "mint";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
          };
        },
        {
          name: "tokenReservesLiquidity";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "sysvarInstruction";
          address: "Sysvar1nstructions1111111111111111111111111";
        },
        {
          name: "metadataProgram";
          address: "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
        },
        {
          name: "rent";
          address: "SysvarRent111111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "symbol";
          type: "string";
        },
        {
          name: "liquidityProgram";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initLendingAdmin";
      discriminator: [203, 185, 241, 165, 56, 254, 33, 9];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "lendingAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  108,
                  101,
                  110,
                  100,
                  105,
                  110,
                  103,
                  95,
                  97,
                  100,
                  109,
                  105,
                  110
                ];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "liquidityProgram";
          type: "pubkey";
        },
        {
          name: "rebalancer";
          type: "pubkey";
        },
        {
          name: "authority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "mint";
      discriminator: [51, 57, 225, 47, 182, 146, 137, 166];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "depositorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "shares";
          type: "u64";
        }
      ];
      returns: "u64";
    },
    {
      name: "mintWithMaxAssets";
      discriminator: [6, 94, 69, 122, 30, 179, 146, 171];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "depositorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "shares";
          type: "u64";
        },
        {
          name: "maxAssets";
          type: "u64";
        }
      ];
      returns: "u64";
    },
    {
      name: "rebalance";
      discriminator: [108, 158, 77, 9, 210, 52, 88, 62];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "depositorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
          writable: true;
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [];
    },
    {
      name: "redeem";
      discriminator: [184, 12, 86, 149, 70, 196, 97, 225];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "ownerTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "shares";
          type: "u64";
        }
      ];
      returns: "u64";
    },
    {
      name: "redeemWithMinAmountOut";
      discriminator: [235, 189, 237, 56, 166, 180, 184, 149];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "ownerTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "shares";
          type: "u64";
        },
        {
          name: "minAmountOut";
          type: "u64";
        }
      ];
    },
    {
      name: "setRewardsRateModel";
      discriminator: [174, 231, 116, 203, 8, 58, 143, 203];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "fTokenMint";
          relations: ["lending"];
        },
        {
          name: "newRewardsRateModel";
        },
        {
          name: "supplyTokenReservesLiquidity";
        }
      ];
      args: [
        {
          name: "mint";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "lendingAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "lendingAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateRate";
      discriminator: [24, 225, 53, 189, 72, 212, 225, 178];
      accounts: [
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "supplyTokenReservesLiquidity"];
        },
        {
          name: "fTokenMint";
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
        },
        {
          name: "rewardsRateModel";
        }
      ];
      args: [];
    },
    {
      name: "updateRebalancer";
      discriminator: [206, 187, 54, 228, 145, 8, 203, 111];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "lendingAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newRebalancer";
          type: "pubkey";
        }
      ];
    },
    {
      name: "withdraw";
      discriminator: [183, 18, 70, 156, 148, 109, 161, 34];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "ownerTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
      returns: "u64";
    },
    {
      name: "withdrawWithMaxSharesBurn";
      discriminator: [47, 197, 183, 171, 239, 18, 245, 171];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "ownerTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "fTokenMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "lendingAdmin";
        },
        {
          name: "lending";
          writable: true;
        },
        {
          name: "mint";
          relations: ["lending", "rewardsRateModel"];
        },
        {
          name: "fTokenMint";
          writable: true;
          relations: ["lending"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "lendingSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "liquidityProgram";
          writable: true;
          relations: ["lendingAdmin"];
        },
        {
          name: "rewardsRateModel";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        },
        {
          name: "maxSharesBurn";
          type: "u64";
        }
      ];
      returns: "u64";
    }
  ];
  accounts: [
    {
      name: "lending";
      discriminator: [135, 199, 82, 16, 249, 131, 182, 241];
    },
    {
      name: "lendingAdmin";
      discriminator: [42, 8, 33, 220, 163, 40, 210, 5];
    },
    {
      name: "lendingRewardsRateModel";
      discriminator: [166, 72, 71, 131, 172, 74, 166, 181];
    },
    {
      name: "tokenReserve";
      discriminator: [21, 18, 59, 135, 120, 20, 31, 12];
    },
    {
      name: "userSupplyPosition";
      discriminator: [202, 219, 136, 118, 61, 177, 21, 146];
    }
  ];
  events: [
    {
      name: "logDeposit";
      discriminator: [176, 243, 1, 56, 142, 206, 1, 106];
    },
    {
      name: "logRebalance";
      discriminator: [90, 67, 219, 41, 181, 118, 132, 9];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    },
    {
      name: "logUpdateRates";
      discriminator: [222, 11, 113, 60, 147, 15, 68, 217];
    },
    {
      name: "logUpdateRebalancer";
      discriminator: [66, 79, 144, 204, 26, 217, 153, 225];
    },
    {
      name: "logUpdateRewards";
      discriminator: [37, 13, 111, 186, 47, 245, 162, 121];
    },
    {
      name: "logWithdraw";
      discriminator: [49, 9, 176, 179, 222, 190, 6, 117];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "fTokenDepositInsignificant";
      msg: "fTokenDepositInsignificant";
    },
    {
      code: 6001;
      name: "fTokenMinAmountOut";
      msg: "fTokenMinAmountOut";
    },
    {
      code: 6002;
      name: "fTokenMaxAmount";
      msg: "fTokenMaxAmount";
    },
    {
      code: 6003;
      name: "fTokenInvalidParams";
      msg: "fTokenInvalidParams";
    },
    {
      code: 6004;
      name: "fTokenRewardsRateModelAlreadySet";
      msg: "fTokenRewardsRateModelAlreadySet";
    },
    {
      code: 6005;
      name: "fTokenMaxAuthCountReached";
      msg: "fTokenMaxAuthCount";
    },
    {
      code: 6006;
      name: "fTokenLiquidityExchangePriceUnexpected";
      msg: "fTokenLiquidityExchangePriceUnexpected";
    },
    {
      code: 6007;
      name: "fTokenCpiToLiquidityFailed";
      msg: "fTokenCpiToLiquidityFailed";
    },
    {
      code: 6008;
      name: "fTokenOnlyAuth";
      msg: "fTokenOnlyAuth";
    },
    {
      code: 6009;
      name: "fTokenOnlyAuthority";
      msg: "fTokenOnlyAuthority";
    },
    {
      code: 6010;
      name: "fTokenOnlyRebalancer";
      msg: "fTokenOnlyRebalancer";
    },
    {
      code: 6011;
      name: "fTokenUserSupplyPositionRequired";
      msg: "fTokenUserSupplyPositionRequired";
    },
    {
      code: 6012;
      name: "fTokenLiquidityProgramMismatch";
      msg: "fTokenLiquidityProgramMismatch";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "lending";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "fTokenMint";
            type: "pubkey";
          },
          {
            name: "lendingId";
            type: "u16";
          },
          {
            name: "decimals";
            docs: ["@dev number of decimals for the fToken, same as ASSET"];
            type: "u8";
          },
          {
            name: "rewardsRateModel";
            docs: [
              "@dev To read PDA of rewards rate model to get_rate instruction"
            ];
            type: "pubkey";
          },
          {
            name: "liquidityExchangePrice";
            docs: [
              "@dev exchange price for the underlying asset in the liquidity protocol (without rewards)"
            ];
            type: "u64";
          },
          {
            name: "tokenExchangePrice";
            docs: [
              "@dev exchange price between fToken and the underlying asset (with rewards)"
            ];
            type: "u64";
          },
          {
            name: "lastUpdateTimestamp";
            docs: [
              "@dev timestamp when exchange prices were updated the last time"
            ];
            type: "u64";
          },
          {
            name: "tokenReservesLiquidity";
            type: "pubkey";
          },
          {
            name: "supplyPositionOnLiquidity";
            type: "pubkey";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "lendingAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "liquidityProgram";
            type: "pubkey";
          },
          {
            name: "rebalancer";
            type: "pubkey";
          },
          {
            name: "nextLendingId";
            type: "u16";
          },
          {
            name: "auths";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "lendingRewardsRateModel";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            docs: ["@dev mint address"];
            type: "pubkey";
          },
          {
            name: "startTvl";
            docs: [
              "@dev tvl below which rewards rate is 0. If current TVL is below this value, triggering `update_rate()` on the fToken",
              "might bring the total TVL above this cut-off."
            ];
            type: "u64";
          },
          {
            name: "duration";
            docs: ["@dev for how long current rewards should run"];
            type: "u64";
          },
          {
            name: "startTime";
            docs: ["@dev when current rewards got started"];
            type: "u64";
          },
          {
            name: "yearlyReward";
            docs: [
              "@dev current annualized reward based on input params (duration, rewardAmount)"
            ];
            type: "u64";
          },
          {
            name: "nextDuration";
            docs: ["@dev Duration for the next rewards phase"];
            type: "u64";
          },
          {
            name: "nextRewardAmount";
            docs: ["@dev Amount of rewards for the next phase"];
            type: "u64";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logDeposit";
      type: {
        kind: "struct";
        fields: [
          {
            name: "sender";
            type: "pubkey";
          },
          {
            name: "receiver";
            type: "pubkey";
          },
          {
            name: "assets";
            type: "u64";
          },
          {
            name: "sharesMinted";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logRebalance";
      type: {
        kind: "struct";
        fields: [
          {
            name: "assets";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateRates";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tokenExchangePrice";
            type: "u64";
          },
          {
            name: "liquidityExchangePrice";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logUpdateRebalancer";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newRebalancer";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateRewards";
      type: {
        kind: "struct";
        fields: [
          {
            name: "rewardsRateModel";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logWithdraw";
      type: {
        kind: "struct";
        fields: [
          {
            name: "sender";
            type: "pubkey";
          },
          {
            name: "receiver";
            type: "pubkey";
          },
          {
            name: "owner";
            type: "pubkey";
          },
          {
            name: "assets";
            type: "u64";
          },
          {
            name: "sharesBurned";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "tokenReserve";
      docs: ["Token configuration and exchange prices"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "vault";
            type: "pubkey";
          },
          {
            name: "borrowRate";
            type: "u16";
          },
          {
            name: "feeOnInterest";
            type: "u16";
          },
          {
            name: "lastUtilization";
            type: "u16";
          },
          {
            name: "lastUpdateTimestamp";
            type: "u64";
          },
          {
            name: "supplyExchangePrice";
            type: "u64";
          },
          {
            name: "borrowExchangePrice";
            type: "u64";
          },
          {
            name: "maxUtilization";
            type: "u16";
          },
          {
            name: "totalSupplyWithInterest";
            type: "u64";
          },
          {
            name: "totalSupplyInterestFree";
            type: "u64";
          },
          {
            name: "totalBorrowWithInterest";
            type: "u64";
          },
          {
            name: "totalBorrowInterestFree";
            type: "u64";
          },
          {
            name: "totalClaimAmount";
            type: "u64";
          },
          {
            name: "interactingProtocol";
            type: "pubkey";
          },
          {
            name: "interactingTimestamp";
            type: "u64";
          },
          {
            name: "interactingBalance";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "userSupplyPosition";
      docs: ["User supply position"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "protocol";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "withInterest";
            type: "u8";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "withdrawalLimit";
            type: "u128";
          },
          {
            name: "lastUpdate";
            type: "u64";
          },
          {
            name: "expandPct";
            type: "u16";
          },
          {
            name: "expandDuration";
            type: "u64";
          },
          {
            name: "baseWithdrawalLimit";
            type: "u64";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/oracle.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/oracle.json`.
 */
export type Oracle = {
  address: "jupnw4B6Eqs7ft6rxpzYLJZYSnrpRgPcr589n5Kv4oc";
  metadata: {
    name: "oracle";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "getBothExchangeRate";
      discriminator: [92, 88, 161, 46, 230, 193, 46, 237];
      accounts: [
        {
          name: "oracle";
        }
      ];
      args: [
        {
          name: "nonce";
          type: "u16";
        }
      ];
    },
    {
      name: "getExchangeRate";
      discriminator: [153, 76, 17, 194, 170, 215, 89, 142];
      accounts: [
        {
          name: "oracle";
        }
      ];
      args: [
        {
          name: "nonce";
          type: "u16";
        }
      ];
      returns: "u128";
    },
    {
      name: "getExchangeRateLiquidate";
      discriminator: [228, 169, 73, 39, 91, 82, 27, 5];
      accounts: [
        {
          name: "oracle";
        }
      ];
      args: [
        {
          name: "nonce";
          type: "u16";
        }
      ];
      returns: "u128";
    },
    {
      name: "getExchangeRateOperate";
      discriminator: [174, 166, 126, 10, 122, 153, 94, 203];
      accounts: [
        {
          name: "oracle";
        }
      ];
      args: [
        {
          name: "nonce";
          type: "u16";
        }
      ];
      returns: "u128";
    },
    {
      name: "initAdmin";
      discriminator: [97, 65, 97, 27, 200, 206, 72, 219];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "oracleAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [111, 114, 97, 99, 108, 101, 95, 97, 100, 109, 105, 110];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "authority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initOracleConfig";
      discriminator: [77, 144, 180, 246, 217, 15, 118, 92];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "oracleAdmin";
        },
        {
          name: "oracle";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [111, 114, 97, 99, 108, 101];
              },
              {
                kind: "arg";
                path: "nonce";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "sources";
          type: {
            vec: {
              defined: {
                name: "sources";
              };
            };
          };
        },
        {
          name: "nonce";
          type: "u16";
        }
      ];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "oracleAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "oracleAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    }
  ];
  accounts: [
    {
      name: "oracle";
      discriminator: [139, 194, 131, 179, 140, 179, 229, 244];
    },
    {
      name: "oracleAdmin";
      discriminator: [239, 232, 8, 20, 254, 63, 25, 246];
    }
  ];
  events: [
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "priceNotValid";
      msg: "priceNotValid";
    },
    {
      code: 6001;
      name: "priceTooOld";
      msg: "priceTooOld";
    },
    {
      code: 6002;
      name: "rateZero";
      msg: "rateZero";
    },
    {
      code: 6003;
      name: "invalidParams";
      msg: "invalidParams";
    },
    {
      code: 6004;
      name: "invalidPythSourceMultiplierAndDivisor";
      msg: "invalidPythSourceMultiplierAndDivisor";
    },
    {
      code: 6005;
      name: "invalidSource";
      msg: "invalidSource";
    },
    {
      code: 6006;
      name: "invalidSourcesLength";
      msg: "invalidSourcesLength";
    },
    {
      code: 6007;
      name: "oracleAdminOnlyAuthority";
      msg: "oracleAdminOnlyAuthority";
    },
    {
      code: 6008;
      name: "oracleAdminOnlyAuth";
      msg: "oracleAdminOnlyAuth";
    },
    {
      code: 6009;
      name: "oracleAdminMaxAuthCountReached";
      msg: "oracleAdminMaxAuthCountReached";
    },
    {
      code: 6010;
      name: "oracleAdminInvalidParams";
      msg: "oracleAdminInvalidParams";
    },
    {
      code: 6011;
      name: "oracleNonceMismatch";
      msg: "oracleNonceMismatch";
    },
    {
      code: 6012;
      name: "priceConfidenceNotSufficient";
      msg: "priceConfidenceNotSufficient";
    },
    {
      code: 6013;
      name: "stakePoolNotRefreshed";
      msg: "stakePoolNotRefreshed";
    },
    {
      code: 6014;
      name: "invalidPrice";
      msg: "invalidPrice";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "oracle";
      type: {
        kind: "struct";
        fields: [
          {
            name: "nonce";
            type: "u16";
          },
          {
            name: "sources";
            type: {
              vec: {
                defined: {
                  name: "sources";
                };
              };
            };
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "oracleAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "auths";
            type: {
              vec: "pubkey";
            };
          }
        ];
      };
    },
    {
      name: "sourceType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "pyth";
          },
          {
            name: "stakePool";
          }
        ];
      };
    },
    {
      name: "sources";
      type: {
        kind: "struct";
        fields: [
          {
            name: "source";
            type: "pubkey";
          },
          {
            name: "invert";
            type: "bool";
          },
          {
            name: "multiplier";
            type: "u128";
          },
          {
            name: "divisor";
            type: "u128";
          },
          {
            name: "sourceType";
            type: {
              defined: {
                name: "sourceType";
              };
            };
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/flashloan.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/flashloan.json`.
 */
export type Flashloan = {
  address: "jupgfSgfuAXv4B6R2Uxu85Z1qdzgju79s6MfZekN6XS";
  metadata: {
    name: "flashloan";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "activateProtocol";
      discriminator: [230, 235, 188, 19, 120, 91, 11, 94];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        }
      ];
      args: [];
    },
    {
      name: "flashloanBorrow";
      discriminator: [103, 19, 78, 24, 240, 9, 135, 63];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        },
        {
          name: "signerBorrowTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
        },
        {
          name: "flashloanTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "flashloanBorrowPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "liquidityProgram";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "instructionSysvar";
          address: "Sysvar1nstructions1111111111111111111111111";
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "flashloanPayback";
      discriminator: [213, 47, 153, 137, 84, 243, 94, 232];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        },
        {
          name: "signerBorrowTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "mint";
        },
        {
          name: "flashloanTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "flashloanBorrowPositionOnLiquidity";
          writable: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "vault";
          writable: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "liquidityProgram";
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "instructionSysvar";
          address: "Sysvar1nstructions1111111111111111111111111";
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "initFlashloanAdmin";
      discriminator: [185, 117, 154, 56, 95, 12, 187, 139];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  102,
                  108,
                  97,
                  115,
                  104,
                  108,
                  111,
                  97,
                  110,
                  95,
                  97,
                  100,
                  109,
                  105,
                  110
                ];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "authority";
          type: "pubkey";
        },
        {
          name: "flashloanFee";
          type: "u16";
        },
        {
          name: "liquidityProgram";
          type: "pubkey";
        }
      ];
    },
    {
      name: "pauseProtocol";
      discriminator: [144, 95, 0, 107, 119, 39, 248, 141];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        }
      ];
      args: [];
    },
    {
      name: "setFlashloanFee";
      discriminator: [120, 248, 221, 70, 84, 216, 0, 149];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "flashloanFee";
          type: "u16";
        }
      ];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "flashloanAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "flashloanAdmin";
      discriminator: [162, 161, 45, 28, 131, 91, 202, 88];
    }
  ];
  events: [
    {
      name: "activateProtocol";
      discriminator: [70, 178, 173, 151, 180, 166, 68, 102];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "pauseProtocol";
      discriminator: [66, 229, 166, 147, 152, 13, 42, 29];
    },
    {
      name: "setFlashloanFee";
      discriminator: [112, 164, 66, 251, 191, 56, 0, 47];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "flashloanInvalidAuthority";
      msg: "flashloanInvalidAuthority";
    },
    {
      code: 6001;
      name: "flashloanFeeTooHigh";
      msg: "flashloanFeeTooHigh";
    },
    {
      code: 6002;
      name: "flashloanInvalidParams";
      msg: "flashloanInvalidParams";
    },
    {
      code: 6003;
      name: "flashloanAlreadyActive";
      msg: "flashloanAlreadyActive";
    },
    {
      code: 6004;
      name: "flashloanAlreadyInactive";
      msg: "flashloanAlreadyInactive";
    },
    {
      code: 6005;
      name: "flashloanCpiToLiquidityFailed";
      msg: "flashloanCpiToLiquidityFailed";
    },
    {
      code: 6006;
      name: "flashloanNotAllowedInThisSlot";
      msg: "flashloanNotAllowedInThisSlot";
    },
    {
      code: 6007;
      name: "flashloanInvalidInstructionSysvar";
      msg: "flashloanInvalidInstructionSysvar";
    },
    {
      code: 6008;
      name: "flashloanInvalidInstructionData";
      msg: "flashloanInvalidInstructionData";
    },
    {
      code: 6009;
      name: "flashloanPaybackNotFound";
      msg: "flashloanPaybackNotFound";
    }
  ];
  types: [
    {
      name: "activateProtocol";
      type: {
        kind: "struct";
        fields: [];
      };
    },
    {
      name: "flashloanAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "liquidityProgram";
            type: "pubkey";
          },
          {
            name: "status";
            type: "bool";
          },
          {
            name: "flashloanFee";
            type: "u16";
          },
          {
            name: "flashloanTimestamp";
            type: "u64";
          },
          {
            name: "isFlashloanActive";
            type: "bool";
          },
          {
            name: "activeFlashloanAmount";
            type: "u64";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "pauseProtocol";
      type: {
        kind: "struct";
        fields: [];
      };
    },
    {
      name: "setFlashloanFee";
      type: {
        kind: "struct";
        fields: [
          {
            name: "flashloanFee";
            type: "u16";
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/merkle_distributor.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/merkle_distributor.json`.
 */
export type MerkleDistributor = {
  address: "jup9FB8aPL62L8SHwhZJnxnV263qQvc9tseGT6AFLn6";
  metadata: {
    name: "merkleDistributor";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "approveRoot";
      discriminator: [167, 152, 175, 193, 218, 188, 184, 23];
      accounts: [
        {
          name: "approver";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        },
        {
          name: "approverRole";
        }
      ];
      args: [
        {
          name: "merkleRoot";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "cycle";
          type: "u32";
        },
        {
          name: "startSlot";
          type: "u32";
        },
        {
          name: "endSlot";
          type: "u32";
        }
      ];
    },
    {
      name: "claim";
      discriminator: [62, 198, 214, 193, 213, 159, 108, 210];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "merkleDistributor";
          writable: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "mint";
          relations: ["merkleDistributor"];
        },
        {
          name: "claimStatus";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [99, 108, 97, 105, 109];
              },
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "arg";
                path: "positionId";
              },
              {
                kind: "arg";
                path: "distributorId";
              }
            ];
          };
        },
        {
          name: "vaultTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "merkleAdmin";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "cumulativeAmount";
          type: "u64";
        },
        {
          name: "positionType";
          type: "u8";
        },
        {
          name: "positionId";
          type: "pubkey";
        },
        {
          name: "distributorId";
          type: "u32";
        },
        {
          name: "cycle";
          type: "u32";
        },
        {
          name: "merkleProof";
          type: {
            vec: {
              array: ["u8", 32];
            };
          };
        },
        {
          name: "metadata";
          type: "bytes";
        }
      ];
    },
    {
      name: "distributeRewards";
      discriminator: [97, 6, 227, 255, 124, 165, 3, 148];
      accounts: [
        {
          name: "distributor";
          writable: true;
          signer: true;
          relations: ["merkleDistributor"];
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        },
        {
          name: "mint";
          relations: ["merkleDistributor"];
        },
        {
          name: "distributorTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "distributor";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "vaultTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "merkleAdmin";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "initDistributor";
      discriminator: [4, 170, 72, 1, 58, 177, 150, 43];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "merkleAdmin";
          writable: true;
        },
        {
          name: "merkleDistributor";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [100, 105, 115, 116, 114, 105, 98, 117, 116, 111, 114];
              },
              {
                kind: "account";
                path: "mint";
              },
              {
                kind: "account";
                path: "merkle_admin.next_distributor_id";
                account: "merkleAdmin";
              }
            ];
          };
        },
        {
          name: "mint";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "params";
          type: {
            defined: {
              name: "initializeParams";
            };
          };
        }
      ];
    },
    {
      name: "initMerkleAdmin";
      discriminator: [130, 19, 59, 140, 26, 4, 117, 19];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "merkleAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  109,
                  101,
                  114,
                  107,
                  108,
                  101,
                  95,
                  97,
                  100,
                  109,
                  105,
                  110
                ];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "authority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initRole";
      discriminator: [24, 82, 229, 76, 200, 87, 242, 26];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "roleAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [114, 111, 108, 101];
              },
              {
                kind: "arg";
                path: "address";
              },
              {
                kind: "arg";
                path: "role";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "address";
          type: "pubkey";
        },
        {
          name: "role";
          type: "u8";
        }
      ];
    },
    {
      name: "pause";
      discriminator: [211, 22, 221, 251, 74, 121, 193, 47];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        }
      ];
      args: [];
    },
    {
      name: "proposeRoot";
      discriminator: [132, 0, 76, 107, 236, 86, 118, 165];
      accounts: [
        {
          name: "proposer";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        },
        {
          name: "proposerRole";
        }
      ];
      args: [
        {
          name: "merkleRoot";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "cycle";
          type: "u32";
        },
        {
          name: "startSlot";
          type: "u32";
        },
        {
          name: "endSlot";
          type: "u32";
        }
      ];
    },
    {
      name: "setStartBlockOfNextCycle";
      discriminator: [168, 83, 36, 171, 34, 95, 238, 81];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        }
      ];
      args: [
        {
          name: "startBlockOfNextCycle";
          type: "u32";
        }
      ];
    },
    {
      name: "unpause";
      discriminator: [169, 144, 4, 38, 10, 141, 188, 255];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        }
      ];
      args: [];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateDistributionConfig";
      discriminator: [162, 95, 24, 240, 144, 247, 117, 22];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        }
      ];
      args: [
        {
          name: "pullFromDistributor";
          type: "bool";
        },
        {
          name: "blocksPerDistribution";
          type: "u32";
        },
        {
          name: "cyclesPerDistribution";
          type: "u32";
        }
      ];
    },
    {
      name: "updateRewardsDistributor";
      discriminator: [250, 201, 40, 213, 158, 61, 253, 147];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "merkleDistributor";
          writable: true;
        }
      ];
      args: [
        {
          name: "distributor";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateRole";
      discriminator: [36, 223, 162, 98, 168, 209, 75, 151];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "merkleAdmin";
        },
        {
          name: "roleAccount";
          writable: true;
        }
      ];
      args: [
        {
          name: "turnOff";
          type: "bool";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "claimStatus";
      discriminator: [22, 183, 249, 157, 247, 95, 150, 96];
    },
    {
      name: "merkleAdmin";
      discriminator: [0, 192, 185, 207, 98, 65, 4, 187];
    },
    {
      name: "merkleDistributor";
      discriminator: [77, 119, 139, 70, 84, 247, 12, 26];
    },
    {
      name: "roleAccount";
      discriminator: [142, 236, 135, 197, 214, 3, 244, 226];
    }
  ];
  events: [
    {
      name: "logClaimed";
      discriminator: [215, 10, 98, 242, 67, 30, 230, 185];
    },
    {
      name: "logDistribution";
      discriminator: [122, 162, 17, 219, 57, 67, 93, 50];
    },
    {
      name: "logDistributionConfigUpdated";
      discriminator: [64, 108, 152, 215, 83, 217, 187, 190];
    },
    {
      name: "logInitRole";
      discriminator: [14, 236, 197, 243, 241, 106, 70, 162];
    },
    {
      name: "logRewardsDistributorUpdated";
      discriminator: [222, 161, 225, 24, 234, 122, 115, 38];
    },
    {
      name: "logRootProposed";
      discriminator: [241, 45, 0, 250, 225, 243, 158, 34];
    },
    {
      name: "logRootUpdated";
      discriminator: [79, 2, 209, 136, 63, 82, 145, 211];
    },
    {
      name: "logStartBlockOfNextCycleUpdated";
      discriminator: [46, 130, 115, 115, 242, 191, 9, 226];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    },
    {
      name: "logUpdateRole";
      discriminator: [138, 23, 252, 139, 73, 226, 226, 166];
    },
    {
      name: "paused";
      discriminator: [172, 248, 5, 253, 49, 255, 255, 232];
    },
    {
      name: "unpaused";
      discriminator: [156, 150, 47, 174, 120, 216, 93, 117];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "invalidParams";
      msg: "invalidParams";
    },
    {
      code: 6001;
      name: "unauthorized";
      msg: "unauthorized";
    },
    {
      code: 6002;
      name: "rewardsPaused";
      msg: "rewardsPaused";
    },
    {
      code: 6003;
      name: "invalidCycle";
      msg: "invalidCycle";
    },
    {
      code: 6004;
      name: "invalidProof";
      msg: "invalidProof";
    },
    {
      code: 6005;
      name: "nothingToClaim";
      msg: "nothingToClaim";
    },
    {
      code: 6006;
      name: "maxAuthCountReached";
      msg: "maxAuthCountReached";
    },
    {
      code: 6007;
      name: "invalidDistributor";
      msg: "invalidDistributor";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "claimStatus";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "recipient";
            type: "pubkey";
          },
          {
            name: "positionId";
            type: "pubkey";
          },
          {
            name: "positionType";
            type: "u8";
          },
          {
            name: "claimedAmount";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "initializeParams";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributionInHours";
            type: "u64";
          },
          {
            name: "cycleInHours";
            type: "u64";
          },
          {
            name: "startBlock";
            type: "u32";
          },
          {
            name: "pullFromDistributor";
            type: "bool";
          },
          {
            name: "vestingTime";
            type: "u32";
          },
          {
            name: "vestingStartTime";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logClaimed";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "cycle";
            type: "u32";
          },
          {
            name: "positionType";
            type: "u8";
          },
          {
            name: "positionId";
            type: "pubkey";
          },
          {
            name: "timestamp";
            type: "u32";
          },
          {
            name: "blockNumber";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logDistribution";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "epoch";
            type: "u32";
          },
          {
            name: "distributor";
            type: "pubkey";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "startCycle";
            type: "u32";
          },
          {
            name: "endCycle";
            type: "u32";
          },
          {
            name: "registrationBlock";
            type: "u32";
          },
          {
            name: "registrationTimestamp";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logDistributionConfigUpdated";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "pullFromDistributor";
            type: "bool";
          },
          {
            name: "blocksPerDistribution";
            type: "u32";
          },
          {
            name: "cyclesPerDistribution";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logInitRole";
      type: {
        kind: "struct";
        fields: [
          {
            name: "address";
            type: "pubkey";
          },
          {
            name: "role";
            type: {
              defined: {
                name: "roleType";
              };
            };
          }
        ];
      };
    },
    {
      name: "logRewardsDistributorUpdated";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "distributor";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logRootProposed";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "cycle";
            type: "u32";
          },
          {
            name: "merkleRoot";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "timestamp";
            type: "u32";
          },
          {
            name: "publishBlock";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logRootUpdated";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "cycle";
            type: "u32";
          },
          {
            name: "merkleRoot";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "timestamp";
            type: "u32";
          },
          {
            name: "publishBlock";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logStartBlockOfNextCycleUpdated";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "startBlockOfNextCycle";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateRole";
      type: {
        kind: "struct";
        fields: [
          {
            name: "address";
            type: "pubkey";
          },
          {
            name: "role";
            type: {
              defined: {
                name: "roleType";
              };
            };
          }
        ];
      };
    },
    {
      name: "merkleAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "auths";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "nextDistributorId";
            type: "u32";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "merkleCycle";
      repr: {
        kind: "c";
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "merkleRoot";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "cycle";
            type: "u32";
          },
          {
            name: "timestamp";
            type: "u32";
          },
          {
            name: "publishBlock";
            type: "u32";
          },
          {
            name: "startSlot";
            type: "u32";
          },
          {
            name: "endSlot";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "merkleDistributor";
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "distributorId";
            type: "u32";
          },
          {
            name: "paused";
            type: "u8";
          },
          {
            name: "distributor";
            type: "pubkey";
          },
          {
            name: "currentMerkleCycle";
            type: {
              defined: {
                name: "merkleCycle";
              };
            };
          },
          {
            name: "pendingMerkleCycle";
            type: {
              defined: {
                name: "merkleCycle";
              };
            };
          },
          {
            name: "previousMerkleRoot";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "cyclesPerDistribution";
            type: "u32";
          },
          {
            name: "blocksPerDistribution";
            type: "u32";
          },
          {
            name: "startBlockOfNextCycle";
            type: "u32";
          },
          {
            name: "endBlockOfLastCycle";
            type: "u32";
          },
          {
            name: "pullFromDistributor";
            type: "u8";
          },
          {
            name: "vestingTime";
            type: "u32";
          },
          {
            name: "vestingStartTime";
            type: "u32";
          },
          {
            name: "totalRewardsCycles";
            type: "u32";
          },
          {
            name: "totalDistributions";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "paused";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "roleAccount";
      type: {
        kind: "struct";
        fields: [
          {
            name: "address";
            type: "pubkey";
          },
          {
            name: "role";
            type: "u8";
          },
          {
            name: "active";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "roleType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "none";
          },
          {
            name: "proposer";
          },
          {
            name: "approver";
          }
        ];
      };
    },
    {
      name: "unpaused";
      type: {
        kind: "struct";
        fields: [
          {
            name: "distributorId";
            type: "u32";
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/vaults.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/vaults.json`.
 */
export type Vaults = {
  address: "jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi";
  metadata: {
    name: "vaults";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "getExchangePrices";
      discriminator: [237, 128, 83, 152, 52, 21, 231, 86];
      accounts: [
        {
          name: "vaultState";
        },
        {
          name: "vaultConfig";
        },
        {
          name: "supplyTokenReserves";
        },
        {
          name: "borrowTokenReserves";
        }
      ];
      args: [];
    },
    {
      name: "initBranch";
      discriminator: [162, 91, 57, 23, 228, 93, 111, 21];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "branch";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [98, 114, 97, 110, 99, 104];
              },
              {
                kind: "arg";
                path: "vaultId";
              },
              {
                kind: "arg";
                path: "branchId";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "branchId";
          type: "u32";
        }
      ];
    },
    {
      name: "initPosition";
      discriminator: [197, 20, 10, 1, 97, 160, 177, 91];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "position";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [112, 111, 115, 105, 116, 105, 111, 110];
              },
              {
                kind: "arg";
                path: "vaultId";
              },
              {
                kind: "arg";
                path: "nextPositionId";
              }
            ];
          };
        },
        {
          name: "positionMint";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  112,
                  111,
                  115,
                  105,
                  116,
                  105,
                  111,
                  110,
                  95,
                  109,
                  105,
                  110,
                  116
                ];
              },
              {
                kind: "arg";
                path: "vaultId";
              },
              {
                kind: "arg";
                path: "nextPositionId";
              }
            ];
          };
        },
        {
          name: "positionTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "const";
                value: [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ];
              },
              {
                kind: "account";
                path: "positionMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "metadataAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [109, 101, 116, 97, 100, 97, 116, 97];
              },
              {
                kind: "const";
                value: [
                  11,
                  112,
                  101,
                  177,
                  227,
                  209,
                  124,
                  69,
                  56,
                  157,
                  82,
                  127,
                  107,
                  4,
                  195,
                  205,
                  88,
                  184,
                  108,
                  115,
                  26,
                  160,
                  253,
                  181,
                  73,
                  182,
                  209,
                  188,
                  3,
                  248,
                  41,
                  70
                ];
              },
              {
                kind: "account";
                path: "positionMint";
              }
            ];
            program: {
              kind: "const";
              value: [
                11,
                112,
                101,
                177,
                227,
                209,
                124,
                69,
                56,
                157,
                82,
                127,
                107,
                4,
                195,
                205,
                88,
                184,
                108,
                115,
                26,
                160,
                253,
                181,
                73,
                182,
                209,
                188,
                3,
                248,
                41,
                70
              ];
            };
          };
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "sysvarInstruction";
          address: "Sysvar1nstructions1111111111111111111111111";
        },
        {
          name: "metadataProgram";
          address: "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
        },
        {
          name: "rent";
          address: "SysvarRent111111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "nextPositionId";
          type: "u32";
        }
      ];
    },
    {
      name: "initTick";
      discriminator: [22, 13, 62, 141, 73, 89, 178, 29];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "tickData";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "tick";
          type: "i32";
        }
      ];
    },
    {
      name: "initTickHasDebtArray";
      discriminator: [206, 108, 146, 245, 20, 0, 141, 208];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "tickHasDebtArray";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  116,
                  105,
                  99,
                  107,
                  95,
                  104,
                  97,
                  115,
                  95,
                  100,
                  101,
                  98,
                  116
                ];
              },
              {
                kind: "arg";
                path: "vaultId";
              },
              {
                kind: "arg";
                path: "index";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "index";
          type: "u8";
        }
      ];
    },
    {
      name: "initTickIdLiquidation";
      discriminator: [56, 110, 121, 169, 152, 241, 86, 183];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "tickData";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "tickIdLiquidation";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "tick";
          type: "i32";
        },
        {
          name: "totalIds";
          type: "u32";
        }
      ];
    },
    {
      name: "initVaultAdmin";
      discriminator: [22, 133, 2, 244, 123, 100, 249, 230];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "vaultAdmin";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [118, 97, 117, 108, 116, 95, 97, 100, 109, 105, 110];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "liquidity";
          type: "pubkey";
        },
        {
          name: "authority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initVaultConfig";
      discriminator: [41, 194, 69, 254, 196, 246, 226, 195];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "vaultAdmin";
          writable: true;
        },
        {
          name: "vaultConfig";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ];
              },
              {
                kind: "arg";
                path: "vaultId";
              }
            ];
          };
        },
        {
          name: "vaultMetadata";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  109,
                  101,
                  116,
                  97,
                  100,
                  97,
                  116,
                  97
                ];
              },
              {
                kind: "arg";
                path: "vaultId";
              }
            ];
          };
        },
        {
          name: "oracle";
        },
        {
          name: "supplyToken";
        },
        {
          name: "borrowToken";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "params";
          type: {
            defined: {
              name: "initVaultConfigParams";
            };
          };
        }
      ];
    },
    {
      name: "initVaultState";
      discriminator: [96, 120, 23, 100, 153, 11, 13, 165];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "vaultAdmin";
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "vaultState";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [118, 97, 117, 108, 116, 95, 115, 116, 97, 116, 101];
              },
              {
                kind: "arg";
                path: "vaultId";
              }
            ];
          };
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        }
      ];
    },
    {
      name: "liquidate";
      discriminator: [223, 179, 226, 125, 48, 46, 39, 74];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "signerTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "borrowTokenProgram";
              },
              {
                kind: "account";
                path: "borrowToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "to";
        },
        {
          name: "toTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "to";
              },
              {
                kind: "account";
                path: "supplyTokenProgram";
              },
              {
                kind: "account";
                path: "supplyToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "vaultConfig";
          docs: [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ];
        },
        {
          name: "vaultState";
          writable: true;
        },
        {
          name: "supplyToken";
        },
        {
          name: "borrowToken";
        },
        {
          name: "oracle";
        },
        {
          name: "newBranch";
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "borrowTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "vaultSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "vaultBorrowPositionOnLiquidity";
          writable: true;
        },
        {
          name: "supplyRateModel";
        },
        {
          name: "borrowRateModel";
        },
        {
          name: "supplyTokenClaimAccount";
          writable: true;
          optional: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "liquidityProgram";
        },
        {
          name: "vaultSupplyTokenAccount";
          writable: true;
        },
        {
          name: "vaultBorrowTokenAccount";
          writable: true;
        },
        {
          name: "supplyTokenProgram";
        },
        {
          name: "borrowTokenProgram";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "oracleProgram";
        }
      ];
      args: [
        {
          name: "debtAmt";
          type: "u64";
        },
        {
          name: "colPerUnitDebt";
          type: "u128";
        },
        {
          name: "absorb";
          type: "bool";
        },
        {
          name: "transferType";
          type: {
            option: {
              defined: {
                name: "transferType";
              };
            };
          };
        },
        {
          name: "remainingAccountsIndices";
          type: "bytes";
        }
      ];
    },
    {
      name: "operate";
      discriminator: [217, 106, 208, 99, 116, 151, 42, 135];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "signerSupplyTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "supplyTokenProgram";
              },
              {
                kind: "account";
                path: "supplyToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "signerBorrowTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "signer";
              },
              {
                kind: "account";
                path: "borrowTokenProgram";
              },
              {
                kind: "account";
                path: "borrowToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipient";
          optional: true;
        },
        {
          name: "recipientBorrowTokenAccount";
          writable: true;
          optional: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "recipient";
              },
              {
                kind: "account";
                path: "borrowTokenProgram";
              },
              {
                kind: "account";
                path: "borrowToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "recipientSupplyTokenAccount";
          writable: true;
          optional: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "recipient";
              },
              {
                kind: "account";
                path: "supplyTokenProgram";
              },
              {
                kind: "account";
                path: "supplyToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "vaultConfig";
          docs: [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ];
        },
        {
          name: "vaultState";
          docs: ["@dev verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyToken";
        },
        {
          name: "borrowToken";
        },
        {
          name: "oracle";
        },
        {
          name: "position";
          writable: true;
        },
        {
          name: "positionTokenAccount";
          docs: ["@dev verification inside instruction logic"];
        },
        {
          name: "currentPositionTick";
          writable: true;
        },
        {
          name: "finalPositionTick";
          writable: true;
        },
        {
          name: "currentPositionTickId";
        },
        {
          name: "finalPositionTickId";
          writable: true;
        },
        {
          name: "newBranch";
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "borrowTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "vaultSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "vaultBorrowPositionOnLiquidity";
          writable: true;
        },
        {
          name: "supplyRateModel";
        },
        {
          name: "borrowRateModel";
        },
        {
          name: "vaultSupplyTokenAccount";
          writable: true;
        },
        {
          name: "vaultBorrowTokenAccount";
          writable: true;
        },
        {
          name: "supplyTokenClaimAccount";
          writable: true;
          optional: true;
        },
        {
          name: "borrowTokenClaimAccount";
          writable: true;
          optional: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "liquidityProgram";
        },
        {
          name: "oracleProgram";
        },
        {
          name: "supplyTokenProgram";
        },
        {
          name: "borrowTokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "newCol";
          type: "i128";
        },
        {
          name: "newDebt";
          type: "i128";
        },
        {
          name: "transferType";
          type: {
            option: {
              defined: {
                name: "transferType";
              };
            };
          };
        },
        {
          name: "remainingAccountsIndices";
          type: "bytes";
        }
      ];
    },
    {
      name: "rebalance";
      discriminator: [108, 158, 77, 9, 210, 52, 88, 62];
      accounts: [
        {
          name: "rebalancer";
          writable: true;
          signer: true;
          relations: ["vaultConfig"];
        },
        {
          name: "rebalancerSupplyTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "rebalancer";
              },
              {
                kind: "account";
                path: "supplyTokenProgram";
              },
              {
                kind: "account";
                path: "supplyToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "rebalancerBorrowTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "rebalancer";
              },
              {
                kind: "account";
                path: "borrowTokenProgram";
              },
              {
                kind: "account";
                path: "borrowToken";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "vaultConfig";
          docs: [
            "@dev mut because this PDA signs the CPI to liquidity program",
            "@dev verification inside instruction logic"
          ];
          writable: true;
        },
        {
          name: "vaultState";
          docs: ["@dev verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyToken";
          relations: ["vaultConfig"];
        },
        {
          name: "borrowToken";
          relations: ["vaultConfig"];
        },
        {
          name: "supplyTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "borrowTokenReservesLiquidity";
          writable: true;
        },
        {
          name: "vaultSupplyPositionOnLiquidity";
          writable: true;
        },
        {
          name: "vaultBorrowPositionOnLiquidity";
          writable: true;
        },
        {
          name: "supplyRateModel";
        },
        {
          name: "borrowRateModel";
        },
        {
          name: "liquidity";
        },
        {
          name: "liquidityProgram";
        },
        {
          name: "vaultSupplyTokenAccount";
          writable: true;
        },
        {
          name: "vaultBorrowTokenAccount";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        },
        {
          name: "supplyTokenProgram";
        },
        {
          name: "borrowTokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        }
      ];
      args: [];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "vaultAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "signer";
          signer: true;
        },
        {
          name: "vaultAdmin";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateBorrowFee";
      discriminator: [251, 124, 35, 148, 202, 167, 157, 65];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "borrowFee";
          type: "u16";
        }
      ];
    },
    {
      name: "updateBorrowRateMagnifier";
      discriminator: [75, 250, 27, 176, 156, 53, 26, 112];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "borrowRateMagnifier";
          type: "i16";
        }
      ];
    },
    {
      name: "updateCollateralFactor";
      discriminator: [244, 83, 227, 215, 220, 82, 201, 221];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "collateralFactor";
          type: "u16";
        }
      ];
    },
    {
      name: "updateCoreSettings";
      discriminator: [101, 84, 9, 11, 60, 104, 149, 234];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "params";
          type: {
            defined: {
              name: "updateCoreSettingsParams";
            };
          };
        }
      ];
    },
    {
      name: "updateLiquidationMaxLimit";
      discriminator: [183, 242, 152, 150, 176, 40, 65, 161];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "liquidationMaxLimit";
          type: "u16";
        }
      ];
    },
    {
      name: "updateLiquidationPenalty";
      discriminator: [21, 168, 167, 206, 98, 206, 69, 32];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "liquidationPenalty";
          type: "u16";
        }
      ];
    },
    {
      name: "updateLiquidationThreshold";
      discriminator: [53, 185, 87, 243, 138, 11, 79, 28];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "liquidationThreshold";
          type: "u16";
        }
      ];
    },
    {
      name: "updateLookupTable";
      discriminator: [221, 59, 30, 246, 106, 223, 137, 55];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultMetadata";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "lookupTable";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateOracle";
      discriminator: [112, 41, 209, 18, 248, 226, 252, 188];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "newOracle";
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        }
      ];
    },
    {
      name: "updateRebalancer";
      discriminator: [206, 187, 54, 228, 145, 8, 203, 111];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "newRebalancer";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateSupplyRateMagnifier";
      discriminator: [175, 59, 117, 196, 211, 170, 22, 12];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "supplyRateMagnifier";
          type: "i16";
        }
      ];
    },
    {
      name: "updateWithdrawGap";
      discriminator: [229, 163, 76, 21, 82, 215, 25, 233];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "vaultAdmin";
        },
        {
          name: "vaultState";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "vaultConfig";
          docs: ["@dev Verification inside instruction logic"];
          writable: true;
        },
        {
          name: "supplyTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        },
        {
          name: "borrowTokenReservesLiquidity";
          docs: ["@dev Verification inside instruction logic"];
        }
      ];
      args: [
        {
          name: "vaultId";
          type: "u16";
        },
        {
          name: "withdrawGap";
          type: "u16";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "branch";
      discriminator: [14, 63, 100, 50, 25, 8, 29, 5];
    },
    {
      name: "oracle";
      discriminator: [139, 194, 131, 179, 140, 179, 229, 244];
    },
    {
      name: "position";
      discriminator: [170, 188, 143, 228, 122, 64, 247, 208];
    },
    {
      name: "tick";
      discriminator: [176, 94, 67, 247, 133, 173, 7, 115];
    },
    {
      name: "tickHasDebtArray";
      discriminator: [91, 232, 60, 29, 124, 103, 49, 252];
    },
    {
      name: "tickIdLiquidation";
      discriminator: [41, 28, 190, 197, 68, 213, 31, 181];
    },
    {
      name: "tokenReserve";
      discriminator: [21, 18, 59, 135, 120, 20, 31, 12];
    },
    {
      name: "userBorrowPosition";
      discriminator: [73, 126, 65, 123, 220, 126, 197, 24];
    },
    {
      name: "userSupplyPosition";
      discriminator: [202, 219, 136, 118, 61, 177, 21, 146];
    },
    {
      name: "vaultAdmin";
      discriminator: [88, 97, 160, 117, 102, 39, 103, 44];
    },
    {
      name: "vaultConfig";
      discriminator: [99, 86, 43, 216, 184, 102, 119, 77];
    },
    {
      name: "vaultMetadata";
      discriminator: [248, 177, 244, 93, 67, 19, 117, 57];
    },
    {
      name: "vaultState";
      discriminator: [228, 196, 82, 165, 98, 210, 235, 152];
    }
  ];
  events: [
    {
      name: "logAbsorb";
      discriminator: [177, 119, 143, 137, 184, 63, 197, 215];
    },
    {
      name: "logInitBranch";
      discriminator: [127, 182, 211, 219, 140, 189, 193, 101];
    },
    {
      name: "logInitTick";
      discriminator: [56, 182, 35, 79, 249, 114, 9, 175];
    },
    {
      name: "logInitTickHasDebtArray";
      discriminator: [15, 134, 113, 2, 251, 206, 30, 129];
    },
    {
      name: "logInitTickIdLiquidation";
      discriminator: [172, 64, 170, 238, 39, 153, 185, 225];
    },
    {
      name: "logInitVaultConfig";
      discriminator: [194, 158, 35, 55, 179, 48, 174, 46];
    },
    {
      name: "logInitVaultState";
      discriminator: [140, 108, 65, 38, 128, 26, 194, 28];
    },
    {
      name: "logLiquidate";
      discriminator: [154, 128, 202, 147, 65, 233, 195, 73];
    },
    {
      name: "logOperate";
      discriminator: [180, 8, 81, 71, 19, 132, 173, 8];
    },
    {
      name: "logRebalance";
      discriminator: [90, 67, 219, 41, 181, 118, 132, 9];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    },
    {
      name: "logUpdateBorrowFee";
      discriminator: [33, 134, 42, 66, 16, 167, 119, 196];
    },
    {
      name: "logUpdateBorrowRateMagnifier";
      discriminator: [186, 23, 46, 117, 57, 111, 107, 51];
    },
    {
      name: "logUpdateCollateralFactor";
      discriminator: [142, 89, 0, 231, 164, 164, 230, 82];
    },
    {
      name: "logUpdateCoreSettings";
      discriminator: [233, 65, 32, 7, 230, 115, 122, 197];
    },
    {
      name: "logUpdateExchangePrices";
      discriminator: [190, 194, 69, 204, 30, 86, 181, 163];
    },
    {
      name: "logUpdateLiquidationMaxLimit";
      discriminator: [73, 32, 49, 0, 234, 86, 150, 94];
    },
    {
      name: "logUpdateLiquidationPenalty";
      discriminator: [42, 132, 67, 48, 209, 133, 77, 83];
    },
    {
      name: "logUpdateLiquidationThreshold";
      discriminator: [211, 71, 215, 239, 159, 238, 71, 219];
    },
    {
      name: "logUpdateOracle";
      discriminator: [251, 163, 219, 57, 30, 152, 177, 10];
    },
    {
      name: "logUpdateRebalancer";
      discriminator: [66, 79, 144, 204, 26, 217, 153, 225];
    },
    {
      name: "logUpdateSupplyRateMagnifier";
      discriminator: [198, 113, 184, 213, 239, 18, 253, 56];
    },
    {
      name: "logUpdateWithdrawGap";
      discriminator: [182, 248, 48, 47, 8, 159, 21, 35];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "vaultNextTickNotFound";
      msg: "vaultNextTickNotFound";
    },
    {
      code: 6001;
      name: "vaultInvalidPositionMint";
      msg: "vaultInvalidPositionMint";
    },
    {
      code: 6002;
      name: "vaultTickIdLiquidationMismatch";
      msg: "vaultTickIdLiquidationMismatch";
    },
    {
      code: 6003;
      name: "vaultInvalidPositionTokenAmount";
      msg: "vaultInvalidPositionTokenAmount";
    },
    {
      code: 6004;
      name: "vaultInvalidRemainingAccountsIndices";
      msg: "vaultInvalidRemainingAccountsIndices";
    },
    {
      code: 6005;
      name: "vaultTickHasDebtVaultIdMismatch";
      msg: "vaultTickHasDebtVaultIdMismatch";
    },
    {
      code: 6006;
      name: "vaultBranchVaultIdMismatch";
      msg: "vaultBranchVaultIdMismatch";
    },
    {
      code: 6007;
      name: "vaultTickVaultIdMismatch";
      msg: "vaultTickVaultIdMismatch";
    },
    {
      code: 6008;
      name: "vaultInvalidDecimals";
      msg: "vaultInvalidDecimals";
    },
    {
      code: 6009;
      name: "vaultInvalidOperateAmount";
      msg: "vaultInvalidOperateAmount";
    },
    {
      code: 6010;
      name: "vaultTickIsEmpty";
      msg: "vaultTickIsEmpty";
    },
    {
      code: 6011;
      name: "vaultPositionAboveCf";
      msg: "vaultPositionAboveCf";
    },
    {
      code: 6012;
      name: "vaultTopTickDoesNotExist";
      msg: "vaultTopTickDoesNotExist";
    },
    {
      code: 6013;
      name: "vaultExcessSlippageLiquidation";
      msg: "vaultExcessSlippageLiquidation";
    },
    {
      code: 6014;
      name: "vaultNotRebalancer";
      msg: "vaultNotRebalancer";
    },
    {
      code: 6015;
      name: "vaultTokenNotInitialized";
      msg: "vaultTokenNotInitialized";
    },
    {
      code: 6016;
      name: "vaultUserCollateralDebtExceed";
      msg: "vaultUserCollateralDebtExceed";
    },
    {
      code: 6017;
      name: "vaultExcessCollateralWithdrawal";
      msg: "vaultExcessCollateralWithdrawal";
    },
    {
      code: 6018;
      name: "vaultExcessDebtPayback";
      msg: "vaultExcessDebtPayback";
    },
    {
      code: 6019;
      name: "vaultWithdrawMoreThanOperateLimit";
      msg: "vaultWithdrawMoreThanOperateLimit";
    },
    {
      code: 6020;
      name: "vaultInvalidLiquidationAmt";
      msg: "vaultInvalidLiquidationAmt";
    },
    {
      code: 6021;
      name: "vaultLiquidationResult";
      msg: "vaultLiquidationResult";
    },
    {
      code: 6022;
      name: "vaultBranchDebtTooLow";
      msg: "vaultBranchDebtTooLow";
    },
    {
      code: 6023;
      name: "vaultTickDebtTooLow";
      msg: "vaultTickDebtTooLow";
    },
    {
      code: 6024;
      name: "vaultLiquidityExchangePriceUnexpected";
      msg: "vaultLiquidityExchangePriceUnexpected";
    },
    {
      code: 6025;
      name: "vaultUserDebtTooLow";
      msg: "vaultUserDebtTooLow";
    },
    {
      code: 6026;
      name: "vaultInvalidPaybackOrDeposit";
      msg: "vaultInvalidPaybackOrDeposit";
    },
    {
      code: 6027;
      name: "vaultInvalidLiquidation";
      msg: "vaultInvalidLiquidation";
    },
    {
      code: 6028;
      name: "vaultNothingToRebalance";
      msg: "vaultNothingToRebalance";
    },
    {
      code: 6029;
      name: "vaultLiquidationReverts";
      msg: "vaultLiquidationReverts";
    },
    {
      code: 6030;
      name: "vaultInvalidOraclePrice";
      msg: "vaultInvalidOraclePrice";
    },
    {
      code: 6031;
      name: "vaultBranchNotFound";
      msg: "vaultBranchNotFound";
    },
    {
      code: 6032;
      name: "vaultTickNotFound";
      msg: "vaultTickNotFound";
    },
    {
      code: 6033;
      name: "vaultTickHasDebtNotFound";
      msg: "vaultTickHasDebtNotFound";
    },
    {
      code: 6034;
      name: "vaultTickMismatch";
      msg: "vaultTickMismatch";
    },
    {
      code: 6035;
      name: "vaultInvalidVaultId";
      msg: "vaultInvalidVaultId";
    },
    {
      code: 6036;
      name: "vaultInvalidNextPositionId";
      msg: "vaultInvalidNextPositionId";
    },
    {
      code: 6037;
      name: "vaultInvalidSupplyMint";
      msg: "vaultInvalidSupplyMint";
    },
    {
      code: 6038;
      name: "vaultInvalidBorrowMint";
      msg: "vaultInvalidBorrowMint";
    },
    {
      code: 6039;
      name: "vaultInvalidOracle";
      msg: "vaultInvalidOracle";
    },
    {
      code: 6040;
      name: "vaultInvalidTick";
      msg: "vaultInvalidTick";
    },
    {
      code: 6041;
      name: "vaultInvalidLiquidityProgram";
      msg: "vaultInvalidLiquidityProgram";
    },
    {
      code: 6042;
      name: "vaultInvalidPositionAuthority";
      msg: "vaultInvalidPositionAuthority";
    },
    {
      code: 6043;
      name: "vaultOracleNotValid";
      msg: "vaultOracleNotValid";
    },
    {
      code: 6044;
      name: "vaultBranchOwnerNotValid";
      msg: "vaultBranchOwnerNotValid";
    },
    {
      code: 6045;
      name: "vaultTickHasDebtOwnerNotValid";
      msg: "vaultTickHasDebtOwnerNotValid";
    },
    {
      code: 6046;
      name: "vaultTickOwnerNotValid";
      msg: "vaultTickDataOwnerNotValid";
    },
    {
      code: 6047;
      name: "vaultLiquidateRemainingAccountsTooShort";
      msg: "vaultLiquidateRemainingAccountsTooShort";
    },
    {
      code: 6048;
      name: "vaultOperateRemainingAccountsTooShort";
      msg: "vaultOperateRemainingAccountsTooShort";
    },
    {
      code: 6049;
      name: "vaultInvalidZerothBranch";
      msg: "vaultInvalidZerothBranch";
    },
    {
      code: 6050;
      name: "vaultCpiToLiquidityFailed";
      msg: "vaultCpyToLiquidityFailed";
    },
    {
      code: 6051;
      name: "vaultCpiToOracleFailed";
      msg: "vaultCpyToOracleFailed";
    },
    {
      code: 6052;
      name: "vaultOnlyAuthority";
      msg: "vaultOnlyAuthority";
    },
    {
      code: 6053;
      name: "vaultNewBranchInvalid";
      msg: "vaultNewBranchInvalid";
    },
    {
      code: 6054;
      name: "vaultTickHasDebtIndexMismatch";
      msg: "vaultTickHasDebtIndexMismatch";
    },
    {
      code: 6055;
      name: "vaultTickHasDebtOutOfRange";
      msg: "vaultTickHasDebtOutOfRange";
    },
    {
      code: 6056;
      name: "vaultUserSupplyPositionRequired";
      msg: "vaultUserSupplyPositionRequired";
    },
    {
      code: 6057;
      name: "vaultClaimAccountRequired";
      msg: "vaultClaimAccountRequired";
    },
    {
      code: 6058;
      name: "vaultRecipientWithdrawAccountRequired";
      msg: "vaultRecipientWithdrawAccountRequired";
    },
    {
      code: 6059;
      name: "vaultRecipientBorrowAccountRequired";
      msg: "vaultRecipientBorrowAccountRequired";
    },
    {
      code: 6060;
      name: "vaultPositionAboveLiquidationThreshold";
      msg: "vaultPositionAboveLiquidationThreshold";
    },
    {
      code: 6061;
      name: "vaultAdminValueAboveLimit";
      msg: "vaultAdminValueAboveLimit";
    },
    {
      code: 6062;
      name: "vaultAdminOnlyAuths";
      msg: "vaultAdminOnlyAuthAccounts";
    },
    {
      code: 6063;
      name: "vaultAdminAddressZeroNotAllowed";
      msg: "vaultAdminAddressZeroNotAllowed";
    },
    {
      code: 6064;
      name: "vaultAdminVaultIdMismatch";
      msg: "vaultAdminVaultIdMismatch";
    },
    {
      code: 6065;
      name: "vaultAdminTotalIdsMismatch";
      msg: "vaultAdminTotalIdsMismatch";
    },
    {
      code: 6066;
      name: "vaultAdminTickMismatch";
      msg: "vaultAdminTickMismatch";
    },
    {
      code: 6067;
      name: "vaultAdminLiquidityProgramMismatch";
      msg: "vaultAdminLiquidityProgramMismatch";
    },
    {
      code: 6068;
      name: "vaultAdminMaxAuthCountReached";
      msg: "vaultAdminMaxAuthCountReached";
    },
    {
      code: 6069;
      name: "vaultAdminInvalidParams";
      msg: "vaultAdminInvalidParams";
    },
    {
      code: 6070;
      name: "vaultAdminOnlyAuthority";
      msg: "vaultAdminOnlyAuthority";
    },
    {
      code: 6071;
      name: "vaultAdminOracleProgramMismatch";
      msg: "vaultAdminOracleProgramMismatch";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "branch";
      docs: ["Branch data structure"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "branchId";
            type: "u32";
          },
          {
            name: "status";
            type: "u8";
          },
          {
            name: "minimaTick";
            type: "i32";
          },
          {
            name: "minimaTickPartials";
            type: "u32";
          },
          {
            name: "debtLiquidity";
            type: "u64";
          },
          {
            name: "debtFactor";
            type: "u64";
          },
          {
            name: "connectedBranchId";
            type: "u32";
          },
          {
            name: "connectedMinimaTick";
            type: "i32";
          }
        ];
      };
    },
    {
      name: "initVaultConfigParams";
      type: {
        kind: "struct";
        fields: [
          {
            name: "supplyRateMagnifier";
            type: "i16";
          },
          {
            name: "borrowRateMagnifier";
            type: "i16";
          },
          {
            name: "collateralFactor";
            type: "u16";
          },
          {
            name: "liquidationThreshold";
            type: "u16";
          },
          {
            name: "liquidationMaxLimit";
            type: "u16";
          },
          {
            name: "withdrawGap";
            type: "u16";
          },
          {
            name: "liquidationPenalty";
            type: "u16";
          },
          {
            name: "borrowFee";
            type: "u16";
          },
          {
            name: "rebalancer";
            type: "pubkey";
          },
          {
            name: "liquidityProgram";
            type: "pubkey";
          },
          {
            name: "oracleProgram";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logAbsorb";
      type: {
        kind: "struct";
        fields: [
          {
            name: "colAmount";
            type: "u64";
          },
          {
            name: "debtAmount";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logInitBranch";
      type: {
        kind: "struct";
        fields: [
          {
            name: "branch";
            type: "pubkey";
          },
          {
            name: "branchId";
            type: "u32";
          }
        ];
      };
    },
    {
      name: "logInitTick";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tick";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logInitTickHasDebtArray";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tickHasDebtArray";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logInitTickIdLiquidation";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tickIdLiquidation";
            type: "pubkey";
          },
          {
            name: "tick";
            type: "i32";
          }
        ];
      };
    },
    {
      name: "logInitVaultConfig";
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultConfig";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logInitVaultState";
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultState";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logLiquidate";
      type: {
        kind: "struct";
        fields: [
          {
            name: "signer";
            type: "pubkey";
          },
          {
            name: "colAmount";
            type: "u64";
          },
          {
            name: "debtAmount";
            type: "u64";
          },
          {
            name: "to";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logOperate";
      type: {
        kind: "struct";
        fields: [
          {
            name: "signer";
            type: "pubkey";
          },
          {
            name: "nftId";
            type: "u32";
          },
          {
            name: "newCol";
            type: "i128";
          },
          {
            name: "newDebt";
            type: "i128";
          },
          {
            name: "to";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logRebalance";
      type: {
        kind: "struct";
        fields: [
          {
            name: "supplyAmt";
            type: "i128";
          },
          {
            name: "borrowAmt";
            type: "i128";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateBorrowFee";
      type: {
        kind: "struct";
        fields: [
          {
            name: "borrowFee";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateBorrowRateMagnifier";
      type: {
        kind: "struct";
        fields: [
          {
            name: "borrowRateMagnifier";
            type: "i16";
          }
        ];
      };
    },
    {
      name: "logUpdateCollateralFactor";
      type: {
        kind: "struct";
        fields: [
          {
            name: "collateralFactor";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateCoreSettings";
      type: {
        kind: "struct";
        fields: [
          {
            name: "supplyRateMagnifier";
            type: "i16";
          },
          {
            name: "borrowRateMagnifier";
            type: "i16";
          },
          {
            name: "collateralFactor";
            type: "u16";
          },
          {
            name: "liquidationThreshold";
            type: "u16";
          },
          {
            name: "liquidationMaxLimit";
            type: "u16";
          },
          {
            name: "withdrawGap";
            type: "u16";
          },
          {
            name: "liquidationPenalty";
            type: "u16";
          },
          {
            name: "borrowFee";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateExchangePrices";
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultSupplyExchangePrice";
            type: "u64";
          },
          {
            name: "vaultBorrowExchangePrice";
            type: "u64";
          },
          {
            name: "liquiditySupplyExchangePrice";
            type: "u64";
          },
          {
            name: "liquidityBorrowExchangePrice";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logUpdateLiquidationMaxLimit";
      type: {
        kind: "struct";
        fields: [
          {
            name: "liquidationMaxLimit";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateLiquidationPenalty";
      type: {
        kind: "struct";
        fields: [
          {
            name: "liquidationPenalty";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateLiquidationThreshold";
      type: {
        kind: "struct";
        fields: [
          {
            name: "liquidationThreshold";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateOracle";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newOracle";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateRebalancer";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newRebalancer";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateSupplyRateMagnifier";
      type: {
        kind: "struct";
        fields: [
          {
            name: "supplyRateMagnifier";
            type: "i16";
          }
        ];
      };
    },
    {
      name: "logUpdateWithdrawGap";
      type: {
        kind: "struct";
        fields: [
          {
            name: "withdrawGap";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "oracle";
      type: {
        kind: "struct";
        fields: [
          {
            name: "nonce";
            type: "u16";
          },
          {
            name: "sources";
            type: {
              vec: {
                defined: {
                  name: "sources";
                };
              };
            };
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "position";
      docs: ["Position data structure"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "nftId";
            type: "u32";
          },
          {
            name: "positionMint";
            type: "pubkey";
          },
          {
            name: "isSupplyOnlyPosition";
            type: "u8";
          },
          {
            name: "tick";
            type: "i32";
          },
          {
            name: "tickId";
            type: "u32";
          },
          {
            name: "supplyAmount";
            type: "u64";
          },
          {
            name: "dustDebtAmount";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "sourceType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "pyth";
          },
          {
            name: "stakePool";
          }
        ];
      };
    },
    {
      name: "sources";
      type: {
        kind: "struct";
        fields: [
          {
            name: "source";
            type: "pubkey";
          },
          {
            name: "invert";
            type: "bool";
          },
          {
            name: "multiplier";
            type: "u128";
          },
          {
            name: "divisor";
            type: "u128";
          },
          {
            name: "sourceType";
            type: {
              defined: {
                name: "sourceType";
              };
            };
          }
        ];
      };
    },
    {
      name: "tick";
      docs: ["Tick data structure"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "tick";
            type: "i32";
          },
          {
            name: "isLiquidated";
            type: "u8";
          },
          {
            name: "totalIds";
            type: "u32";
          },
          {
            name: "rawDebt";
            type: "u64";
          },
          {
            name: "isFullyLiquidated";
            type: "u8";
          },
          {
            name: "liquidationBranchId";
            type: "u32";
          },
          {
            name: "debtFactor";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "tickHasDebt";
      docs: [
        "Tick has debt structure",
        "Each TickHasDebt can track 8 * 256 = 2048 ticks",
        "children_bits has 32 bytes = 256 bits total",
        "Each map within the array covers 256 ticks"
      ];
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "childrenBits";
            type: {
              array: ["u8", 32];
            };
          }
        ];
      };
    },
    {
      name: "tickHasDebtArray";
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "index";
            type: "u8";
          },
          {
            name: "tickHasDebt";
            docs: [
              "Each array contains 8 TickHasDebt structs",
              "Each TickHasDebt covers 256 ticks",
              "Total: 8 * 256 = 2048 ticks per TickHasDebtArray"
            ];
            type: {
              array: [
                {
                  defined: {
                    name: "tickHasDebt";
                  };
                },
                8
              ];
            };
          }
        ];
      };
    },
    {
      name: "tickIdLiquidation";
      docs: ["Tick ID liquidation data"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "tick";
            type: "i32";
          },
          {
            name: "tickMap";
            type: "u32";
          },
          {
            name: "isFullyLiquidated1";
            type: "u8";
          },
          {
            name: "liquidationBranchId1";
            type: "u32";
          },
          {
            name: "debtFactor1";
            type: "u64";
          },
          {
            name: "isFullyLiquidated2";
            type: "u8";
          },
          {
            name: "liquidationBranchId2";
            type: "u32";
          },
          {
            name: "debtFactor2";
            type: "u64";
          },
          {
            name: "isFullyLiquidated3";
            type: "u8";
          },
          {
            name: "liquidationBranchId3";
            type: "u32";
          },
          {
            name: "debtFactor3";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "tokenReserve";
      docs: ["Token configuration and exchange prices"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "vault";
            type: "pubkey";
          },
          {
            name: "borrowRate";
            type: "u16";
          },
          {
            name: "feeOnInterest";
            type: "u16";
          },
          {
            name: "lastUtilization";
            type: "u16";
          },
          {
            name: "lastUpdateTimestamp";
            type: "u64";
          },
          {
            name: "supplyExchangePrice";
            type: "u64";
          },
          {
            name: "borrowExchangePrice";
            type: "u64";
          },
          {
            name: "maxUtilization";
            type: "u16";
          },
          {
            name: "totalSupplyWithInterest";
            type: "u64";
          },
          {
            name: "totalSupplyInterestFree";
            type: "u64";
          },
          {
            name: "totalBorrowWithInterest";
            type: "u64";
          },
          {
            name: "totalBorrowInterestFree";
            type: "u64";
          },
          {
            name: "totalClaimAmount";
            type: "u64";
          },
          {
            name: "interactingProtocol";
            type: "pubkey";
          },
          {
            name: "interactingTimestamp";
            type: "u64";
          },
          {
            name: "interactingBalance";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "transferType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "skip";
          },
          {
            name: "direct";
          },
          {
            name: "claim";
          }
        ];
      };
    },
    {
      name: "updateCoreSettingsParams";
      type: {
        kind: "struct";
        fields: [
          {
            name: "supplyRateMagnifier";
            type: "i16";
          },
          {
            name: "borrowRateMagnifier";
            type: "i16";
          },
          {
            name: "collateralFactor";
            type: "u16";
          },
          {
            name: "liquidationThreshold";
            type: "u16";
          },
          {
            name: "liquidationMaxLimit";
            type: "u16";
          },
          {
            name: "withdrawGap";
            type: "u16";
          },
          {
            name: "liquidationPenalty";
            type: "u16";
          },
          {
            name: "borrowFee";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "userBorrowPosition";
      docs: ["User borrow position"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "protocol";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "withInterest";
            type: "u8";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "debtCeiling";
            type: "u64";
          },
          {
            name: "lastUpdate";
            type: "u64";
          },
          {
            name: "expandPct";
            type: "u16";
          },
          {
            name: "expandDuration";
            type: "u32";
          },
          {
            name: "baseDebtCeiling";
            type: "u64";
          },
          {
            name: "maxDebtCeiling";
            type: "u64";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "userSupplyPosition";
      docs: ["User supply position"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "protocol";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "withInterest";
            type: "u8";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "withdrawalLimit";
            type: "u128";
          },
          {
            name: "lastUpdate";
            type: "u64";
          },
          {
            name: "expandPct";
            type: "u16";
          },
          {
            name: "expandDuration";
            type: "u64";
          },
          {
            name: "baseWithdrawalLimit";
            type: "u64";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "vaultAdmin";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "liquidityProgram";
            type: "pubkey";
          },
          {
            name: "nextVaultId";
            type: "u16";
          },
          {
            name: "auths";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "vaultConfig";
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "supplyRateMagnifier";
            type: "i16";
          },
          {
            name: "borrowRateMagnifier";
            type: "i16";
          },
          {
            name: "collateralFactor";
            type: "u16";
          },
          {
            name: "liquidationThreshold";
            type: "u16";
          },
          {
            name: "liquidationMaxLimit";
            type: "u16";
          },
          {
            name: "withdrawGap";
            type: "u16";
          },
          {
            name: "liquidationPenalty";
            type: "u16";
          },
          {
            name: "borrowFee";
            type: "u16";
          },
          {
            name: "oracle";
            type: "pubkey";
          },
          {
            name: "rebalancer";
            type: "pubkey";
          },
          {
            name: "liquidityProgram";
            type: "pubkey";
          },
          {
            name: "oracleProgram";
            type: "pubkey";
          },
          {
            name: "supplyToken";
            type: "pubkey";
          },
          {
            name: "borrowToken";
            type: "pubkey";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "vaultMetadata";
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "lookupTable";
            type: "pubkey";
          },
          {
            name: "supplyMintDecimals";
            type: "u8";
          },
          {
            name: "borrowMintDecimals";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "vaultState";
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "vaultId";
            type: "u16";
          },
          {
            name: "branchLiquidated";
            type: "u8";
          },
          {
            name: "topmostTick";
            type: "i32";
          },
          {
            name: "currentBranchId";
            type: "u32";
          },
          {
            name: "totalBranchId";
            type: "u32";
          },
          {
            name: "totalSupply";
            type: "u64";
          },
          {
            name: "totalBorrow";
            type: "u64";
          },
          {
            name: "totalPositions";
            type: "u32";
          },
          {
            name: "absorbedDebtAmount";
            type: "u128";
          },
          {
            name: "absorbedColAmount";
            type: "u128";
          },
          {
            name: "absorbedDustDebt";
            type: "u64";
          },
          {
            name: "liquiditySupplyExchangePrice";
            type: "u64";
          },
          {
            name: "liquidityBorrowExchangePrice";
            type: "u64";
          },
          {
            name: "vaultSupplyExchangePrice";
            type: "u64";
          },
          {
            name: "vaultBorrowExchangePrice";
            type: "u64";
          },
          {
            name: "nextPositionId";
            type: "u32";
          },
          {
            name: "lastUpdateTimestamp";
            type: "u64";
          }
        ];
      };
    }
  ];
};

```
---
## `target/types/liquidity.ts`

```typescript
/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/liquidity.json`.
 */
export type Liquidity = {
  address: "jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC";
  metadata: {
    name: "liquidity";
    version: "0.1.0";
    spec: "0.1.0";
    description: "Created with Anchor";
  };
  instructions: [
    {
      name: "changeStatus";
      discriminator: [236, 145, 131, 228, 227, 17, 192, 255];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "authList";
        }
      ];
      args: [
        {
          name: "status";
          type: "bool";
        }
      ];
    },
    {
      name: "claim";
      discriminator: [62, 198, 214, 193, 213, 159, 108, 210];
      accounts: [
        {
          name: "user";
          signer: true;
          relations: ["claimAccount"];
        },
        {
          name: "liquidity";
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "mint";
          relations: ["tokenReserve", "claimAccount"];
        },
        {
          name: "recipientTokenAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "arg";
                path: "recipient";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "vault";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "liquidity";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
          relations: ["tokenReserve"];
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        }
      ];
      args: [
        {
          name: "recipient";
          type: "pubkey";
        }
      ];
    },
    {
      name: "closeClaimAccount";
      discriminator: [241, 146, 203, 216, 58, 222, 91, 118];
      accounts: [
        {
          name: "user";
          writable: true;
          signer: true;
          relations: ["claimAccount"];
        },
        {
          name: "claimAccount";
          writable: true;
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "mint";
          type: "pubkey";
        }
      ];
    },
    {
      name: "collectRevenue";
      discriminator: [87, 96, 211, 36, 240, 43, 246, 87];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "authList";
        },
        {
          name: "mint";
          relations: ["tokenReserve"];
        },
        {
          name: "revenueCollectorAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "revenueCollector";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "revenueCollector";
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "vault";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "liquidity";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
          relations: ["tokenReserve"];
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [];
    },
    {
      name: "initClaimAccount";
      discriminator: [112, 141, 47, 170, 42, 99, 144, 145];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "claimAccount";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [117, 115, 101, 114, 95, 99, 108, 97, 105, 109];
              },
              {
                kind: "arg";
                path: "user";
              },
              {
                kind: "arg";
                path: "mint";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "mint";
          type: "pubkey";
        },
        {
          name: "user";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initLiquidity";
      discriminator: [95, 189, 216, 183, 188, 62, 244, 108];
      accounts: [
        {
          name: "signer";
          writable: true;
          signer: true;
        },
        {
          name: "liquidity";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [108, 105, 113, 117, 105, 100, 105, 116, 121];
              }
            ];
          };
        },
        {
          name: "authList";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [97, 117, 116, 104, 95, 108, 105, 115, 116];
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "authority";
          type: "pubkey";
        },
        {
          name: "revenueCollector";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initNewProtocol";
      discriminator: [193, 147, 5, 32, 138, 135, 213, 158];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "userSupplyPosition";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  117,
                  115,
                  101,
                  114,
                  95,
                  115,
                  117,
                  112,
                  112,
                  108,
                  121,
                  95,
                  112,
                  111,
                  115,
                  105,
                  116,
                  105,
                  111,
                  110
                ];
              },
              {
                kind: "arg";
                path: "supplyMint";
              },
              {
                kind: "arg";
                path: "protocol";
              }
            ];
          };
        },
        {
          name: "userBorrowPosition";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [
                  117,
                  115,
                  101,
                  114,
                  95,
                  98,
                  111,
                  114,
                  114,
                  111,
                  119,
                  95,
                  112,
                  111,
                  115,
                  105,
                  116,
                  105,
                  111,
                  110
                ];
              },
              {
                kind: "arg";
                path: "borrowMint";
              },
              {
                kind: "arg";
                path: "protocol";
              }
            ];
          };
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [
        {
          name: "supplyMint";
          type: "pubkey";
        },
        {
          name: "borrowMint";
          type: "pubkey";
        },
        {
          name: "protocol";
          type: "pubkey";
        }
      ];
    },
    {
      name: "initTokenReserve";
      discriminator: [228, 235, 65, 129, 159, 15, 6, 84];
      accounts: [
        {
          name: "authority";
          writable: true;
          signer: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "authList";
        },
        {
          name: "mint";
        },
        {
          name: "vault";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "liquidity";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "rateModel";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [114, 97, 116, 101, 95, 109, 111, 100, 101, 108];
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
          };
        },
        {
          name: "tokenReserve";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "const";
                value: [114, 101, 115, 101, 114, 118, 101];
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
          };
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "systemProgram";
          address: "11111111111111111111111111111111";
        }
      ];
      args: [];
    },
    {
      name: "operate";
      discriminator: [217, 106, 208, 99, 116, 151, 42, 135];
      accounts: [
        {
          name: "protocol";
          signer: true;
          relations: ["userSupplyPosition", "userBorrowPosition"];
        },
        {
          name: "liquidity";
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "mint";
          relations: [
            "tokenReserve",
            "rateModel",
            "borrowClaimAccount",
            "withdrawClaimAccount"
          ];
        },
        {
          name: "vault";
          writable: true;
          pda: {
            seeds: [
              {
                kind: "account";
                path: "liquidity";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
          relations: ["tokenReserve"];
        },
        {
          name: "userSupplyPosition";
          writable: true;
          optional: true;
        },
        {
          name: "userBorrowPosition";
          writable: true;
          optional: true;
        },
        {
          name: "rateModel";
        },
        {
          name: "withdrawToAccount";
          writable: true;
          optional: true;
          pda: {
            seeds: [
              {
                kind: "arg";
                path: "withdrawTo";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "borrowToAccount";
          writable: true;
          optional: true;
          pda: {
            seeds: [
              {
                kind: "arg";
                path: "borrowTo";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "account";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
        },
        {
          name: "borrowClaimAccount";
          writable: true;
          optional: true;
        },
        {
          name: "withdrawClaimAccount";
          writable: true;
          optional: true;
        },
        {
          name: "tokenProgram";
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        }
      ];
      args: [
        {
          name: "supplyAmount";
          type: "i128";
        },
        {
          name: "borrowAmount";
          type: "i128";
        },
        {
          name: "withdrawTo";
          type: "pubkey";
        },
        {
          name: "borrowTo";
          type: "pubkey";
        },
        {
          name: "transferType";
          type: {
            defined: {
              name: "transferType";
            };
          };
        }
      ];
    },
    {
      name: "pauseUser";
      discriminator: [18, 63, 43, 94, 239, 53, 101, 14];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "userSupplyPosition";
          writable: true;
        },
        {
          name: "userBorrowPosition";
          writable: true;
        }
      ];
      args: [
        {
          name: "protocol";
          type: "pubkey";
        },
        {
          name: "supplyMint";
          type: "pubkey";
        },
        {
          name: "borrowMint";
          type: "pubkey";
        },
        {
          name: "supplyStatus";
          type: {
            option: "u8";
          };
        },
        {
          name: "borrowStatus";
          type: {
            option: "u8";
          };
        }
      ];
    },
    {
      name: "preOperate";
      discriminator: [129, 205, 158, 155, 198, 155, 72, 133];
      accounts: [
        {
          name: "protocol";
          signer: true;
          relations: ["userSupplyPosition", "userBorrowPosition"];
        },
        {
          name: "liquidity";
        },
        {
          name: "userSupplyPosition";
          optional: true;
        },
        {
          name: "userBorrowPosition";
          optional: true;
        },
        {
          name: "vault";
          pda: {
            seeds: [
              {
                kind: "account";
                path: "liquidity";
              },
              {
                kind: "account";
                path: "tokenProgram";
              },
              {
                kind: "arg";
                path: "mint";
              }
            ];
            program: {
              kind: "const";
              value: [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ];
            };
          };
          relations: ["tokenReserve"];
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "associatedTokenProgram";
          address: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
        },
        {
          name: "tokenProgram";
        }
      ];
      args: [
        {
          name: "mint";
          type: "pubkey";
        }
      ];
    },
    {
      name: "unpauseUser";
      discriminator: [71, 115, 128, 252, 182, 126, 234, 62];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "userSupplyPosition";
          writable: true;
        },
        {
          name: "userBorrowPosition";
          writable: true;
        }
      ];
      args: [
        {
          name: "protocol";
          type: "pubkey";
        },
        {
          name: "supplyMint";
          type: "pubkey";
        },
        {
          name: "borrowMint";
          type: "pubkey";
        },
        {
          name: "supplyStatus";
          type: {
            option: "u8";
          };
        },
        {
          name: "borrowStatus";
          type: {
            option: "u8";
          };
        }
      ];
    },
    {
      name: "updateAuthority";
      discriminator: [32, 46, 64, 28, 149, 75, 243, 88];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "liquidity";
          writable: true;
        },
        {
          name: "authList";
          writable: true;
        }
      ];
      args: [
        {
          name: "newAuthority";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateAuths";
      discriminator: [93, 96, 178, 156, 57, 117, 253, 209];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "authList";
          writable: true;
        }
      ];
      args: [
        {
          name: "authStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateExchangePrice";
      discriminator: [239, 244, 10, 248, 116, 25, 53, 150];
      accounts: [
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "rateModel";
          writable: true;
        }
      ];
      args: [
        {
          name: "mint";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateGuardians";
      discriminator: [43, 62, 250, 138, 141, 117, 132, 97];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "liquidity";
        },
        {
          name: "authList";
          writable: true;
        }
      ];
      args: [
        {
          name: "guardianStatus";
          type: {
            vec: {
              defined: {
                name: "addressBool";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateRateDataV1";
      discriminator: [6, 20, 34, 122, 22, 150, 180, 22];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "rateModel";
          writable: true;
        },
        {
          name: "mint";
          relations: ["rateModel", "tokenReserve"];
        },
        {
          name: "tokenReserve";
          writable: true;
        }
      ];
      args: [
        {
          name: "rateData";
          type: {
            defined: {
              name: "rateDataV1Params";
            };
          };
        }
      ];
    },
    {
      name: "updateRateDataV2";
      discriminator: [116, 73, 53, 146, 216, 45, 228, 124];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "rateModel";
          writable: true;
        },
        {
          name: "mint";
          relations: ["rateModel", "tokenReserve"];
        },
        {
          name: "tokenReserve";
          writable: true;
        }
      ];
      args: [
        {
          name: "rateData";
          type: {
            defined: {
              name: "rateDataV2Params";
            };
          };
        }
      ];
    },
    {
      name: "updateRevenueCollector";
      discriminator: [167, 142, 124, 240, 220, 113, 141, 59];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "liquidity";
          writable: true;
        }
      ];
      args: [
        {
          name: "revenueCollector";
          type: "pubkey";
        }
      ];
    },
    {
      name: "updateTokenConfig";
      discriminator: [231, 122, 181, 79, 255, 79, 144, 167];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "rateModel";
          writable: true;
        },
        {
          name: "mint";
          relations: ["rateModel", "tokenReserve"];
        },
        {
          name: "tokenReserve";
          writable: true;
        }
      ];
      args: [
        {
          name: "tokenConfig";
          type: {
            defined: {
              name: "tokenConfig";
            };
          };
        }
      ];
    },
    {
      name: "updateUserBorrowConfig";
      discriminator: [100, 176, 201, 174, 247, 2, 54, 168];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "protocol";
          relations: ["userBorrowPosition"];
        },
        {
          name: "authList";
        },
        {
          name: "rateModel";
        },
        {
          name: "mint";
          relations: ["rateModel", "tokenReserve", "userBorrowPosition"];
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "userBorrowPosition";
          writable: true;
        }
      ];
      args: [
        {
          name: "userBorrowConfig";
          type: {
            defined: {
              name: "userBorrowConfig";
            };
          };
        }
      ];
    },
    {
      name: "updateUserClass";
      discriminator: [12, 206, 68, 135, 63, 212, 48, 119];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
          writable: true;
        }
      ];
      args: [
        {
          name: "userClass";
          type: {
            vec: {
              defined: {
                name: "addressU8";
              };
            };
          };
        }
      ];
    },
    {
      name: "updateUserSupplyConfig";
      discriminator: [217, 239, 225, 218, 33, 49, 234, 183];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "protocol";
          relations: ["userSupplyPosition"];
        },
        {
          name: "authList";
        },
        {
          name: "rateModel";
        },
        {
          name: "mint";
          relations: ["rateModel", "tokenReserve", "userSupplyPosition"];
        },
        {
          name: "tokenReserve";
          writable: true;
        },
        {
          name: "userSupplyPosition";
          writable: true;
        }
      ];
      args: [
        {
          name: "userSupplyConfig";
          type: {
            defined: {
              name: "userSupplyConfig";
            };
          };
        }
      ];
    },
    {
      name: "updateUserWithdrawalLimit";
      discriminator: [162, 9, 186, 9, 213, 30, 173, 78];
      accounts: [
        {
          name: "authority";
          signer: true;
        },
        {
          name: "authList";
        },
        {
          name: "userSupplyPosition";
          writable: true;
        }
      ];
      args: [
        {
          name: "newLimit";
          type: "u128";
        },
        {
          name: "protocol";
          type: "pubkey";
        },
        {
          name: "mint";
          type: "pubkey";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "authorizationList";
      discriminator: [19, 157, 117, 43, 236, 167, 251, 69];
    },
    {
      name: "liquidity";
      discriminator: [54, 252, 249, 226, 137, 172, 121, 58];
    },
    {
      name: "rateModel";
      discriminator: [94, 3, 203, 219, 107, 137, 4, 162];
    },
    {
      name: "tokenReserve";
      discriminator: [21, 18, 59, 135, 120, 20, 31, 12];
    },
    {
      name: "userBorrowPosition";
      discriminator: [73, 126, 65, 123, 220, 126, 197, 24];
    },
    {
      name: "userClaim";
      discriminator: [228, 142, 195, 181, 228, 147, 32, 209];
    },
    {
      name: "userSupplyPosition";
      discriminator: [202, 219, 136, 118, 61, 177, 21, 146];
    }
  ];
  events: [
    {
      name: "logBorrowRateCap";
      discriminator: [156, 131, 232, 94, 254, 156, 14, 117];
    },
    {
      name: "logChangeStatus";
      discriminator: [89, 77, 37, 172, 141, 31, 74, 42];
    },
    {
      name: "logClaim";
      discriminator: [238, 50, 157, 85, 151, 58, 231, 45];
    },
    {
      name: "logCollectRevenue";
      discriminator: [64, 198, 22, 194, 123, 87, 166, 82];
    },
    {
      name: "logOperate";
      discriminator: [180, 8, 81, 71, 19, 132, 173, 8];
    },
    {
      name: "logPauseUser";
      discriminator: [100, 17, 114, 224, 180, 30, 52, 170];
    },
    {
      name: "logUnpauseUser";
      discriminator: [170, 91, 132, 96, 179, 77, 168, 26];
    },
    {
      name: "logUpdateAuthority";
      discriminator: [150, 152, 157, 143, 6, 135, 193, 101];
    },
    {
      name: "logUpdateAuths";
      discriminator: [88, 80, 109, 48, 111, 203, 76, 251];
    },
    {
      name: "logUpdateExchangePrices";
      discriminator: [190, 194, 69, 204, 30, 86, 181, 163];
    },
    {
      name: "logUpdateGuardians";
      discriminator: [231, 28, 191, 51, 53, 140, 79, 142];
    },
    {
      name: "logUpdateRateDataV1";
      discriminator: [30, 102, 131, 192, 0, 30, 85, 223];
    },
    {
      name: "logUpdateRateDataV2";
      discriminator: [206, 53, 195, 70, 113, 211, 92, 129];
    },
    {
      name: "logUpdateRevenueCollector";
      discriminator: [44, 143, 80, 250, 211, 147, 180, 159];
    },
    {
      name: "logUpdateTokenConfigs";
      discriminator: [24, 205, 191, 130, 47, 40, 233, 218];
    },
    {
      name: "logUpdateUserBorrowConfigs";
      discriminator: [210, 251, 242, 159, 205, 33, 154, 74];
    },
    {
      name: "logUpdateUserClass";
      discriminator: [185, 193, 106, 248, 11, 53, 0, 136];
    },
    {
      name: "logUpdateUserSupplyConfigs";
      discriminator: [142, 160, 21, 90, 87, 88, 18, 51];
    },
    {
      name: "logUpdateUserWithdrawalLimit";
      discriminator: [114, 131, 152, 189, 120, 253, 88, 105];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "userClassNotPausable";
      msg: "adminModuleUserClassNotPausable";
    },
    {
      code: 6001;
      name: "userClassNotFound";
      msg: "adminModuleUserClassNotFound";
    },
    {
      code: 6002;
      name: "userAlreadyPaused";
      msg: "adminModuleUserAlreadyPaused";
    },
    {
      code: 6003;
      name: "userAlreadyUnpaused";
      msg: "adminModuleUserAlreadyUnpaused";
    },
    {
      code: 6004;
      name: "onlyLiquidityAuthority";
      msg: "adminModuleOnlyLiquidityAuthority";
    },
    {
      code: 6005;
      name: "onlyAuth";
      msg: "adminModuleOnlyAuth";
    },
    {
      code: 6006;
      name: "onlyGuardians";
      msg: "adminModuleOnlyGuardians";
    },
    {
      code: 6007;
      name: "invalidParams";
      msg: "adminModuleInvalidParams";
    },
    {
      code: 6008;
      name: "invalidConfigOrder";
      msg: "adminModuleInvalidConfigOrder";
    },
    {
      code: 6009;
      name: "statusAlreadySet";
      msg: "adminModuleStatusAlreadySet";
    },
    {
      code: 6010;
      name: "limitsCannotBeZero";
      msg: "adminModuleLimitsCanNotBeZero";
    },
    {
      code: 6011;
      name: "maxAuthCountReached";
      msg: "adminModuleMaxAuthCount";
    },
    {
      code: 6012;
      name: "maxUserClassesReached";
      msg: "adminModuleMaxUserClasses";
    },
    {
      code: 6013;
      name: "insufficientBalance";
      msg: "userModuleInsufficientBalance";
    },
    {
      code: 6014;
      name: "userSupplyPositionRequired";
      msg: "userModuleUserSupplyPositionRequired";
    },
    {
      code: 6015;
      name: "userBorrowPositionRequired";
      msg: "userModuleUserBorrowPositionRequired";
    },
    {
      code: 6016;
      name: "claimAccountRequired";
      msg: "userModuleClaimAccountRequired";
    },
    {
      code: 6017;
      name: "withdrawToAccountRequired";
      msg: "userModuleWithdrawToAccountRequired";
    },
    {
      code: 6018;
      name: "borrowToAccountRequired";
      msg: "userModuleBorrowToAccountRequired";
    },
    {
      code: 6019;
      name: "invalidClaimAmount";
      msg: "userModuleInvalidClaimAmount";
    },
    {
      code: 6020;
      name: "noAmountToClaim";
      msg: "userModuleNoAmountToClaim";
    },
    {
      code: 6021;
      name: "amountNotZero";
      msg: "userModuleAmountNotZero";
    },
    {
      code: 6022;
      name: "valueOverflow";
      msg: "userModuleValueOverflow";
    },
    {
      code: 6023;
      name: "invalidTransferType";
      msg: "userModuleInvalidTransferType";
    },
    {
      code: 6024;
      name: "mintMismatch";
      msg: "userModuleMintMismatch";
    },
    {
      code: 6025;
      name: "userNotDefined";
      msg: "userModuleUserNotDefined";
    },
    {
      code: 6026;
      name: "invalidUserClaim";
      msg: "userModuleInvalidUserClaim";
    },
    {
      code: 6027;
      name: "userPaused";
      msg: "userModuleUserPaused";
    },
    {
      code: 6028;
      name: "withdrawalLimitReached";
      msg: "userModuleWithdrawalLimitReached";
    },
    {
      code: 6029;
      name: "borrowLimitReached";
      msg: "userModuleBorrowLimitReached";
    },
    {
      code: 6030;
      name: "operateAmountsNearlyZero";
      msg: "userModuleOperateAmountsZero";
    },
    {
      code: 6031;
      name: "operateAmountTooBig";
      msg: "userModuleOperateAmountsTooBig";
    },
    {
      code: 6032;
      name: "operateAmountsInsufficient";
      msg: "userModuleOperateAmountsInsufficient";
    },
    {
      code: 6033;
      name: "transferAmountOutOfBounds";
      msg: "userModuleTransferAmountOutOfBounds";
    },
    {
      code: 6034;
      name: "forbiddenOperateCall";
      msg: "forbiddenOperateCall";
    },
    {
      code: 6035;
      name: "maxUtilizationReached";
      msg: "userModuleMaxUtilizationReached";
    },
    {
      code: 6036;
      name: "valueOverflowTotalSupply";
      msg: "userModuleValueOverflowTotalSupply";
    },
    {
      code: 6037;
      name: "valueOverflowTotalBorrow";
      msg: "userModuleValueOverflowTotalBorrow";
    },
    {
      code: 6038;
      name: "depositExpected";
      msg: "userModuleDepositExpected";
    },
    {
      code: 6039;
      name: "exchangePriceZero";
      msg: "liquidityCalcsExchangePriceZero";
    },
    {
      code: 6040;
      name: "unsupportedRateVersion";
      msg: "liquidityCalcsUnsupportedRateVersion";
    },
    {
      code: 6041;
      name: "borrowRateNegative";
      msg: "liquidityCalcsBorrowRateNegative";
    },
    {
      code: 6042;
      name: "protocolLockdown";
      msg: "protocolLockdown";
    }
  ];
  types: [
    {
      name: "addressBool";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "addressU8";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "value";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "authorizationList";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authUsers";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "guardians";
            type: {
              vec: "pubkey";
            };
          },
          {
            name: "userClasses";
            type: {
              vec: {
                defined: {
                  name: "userClass";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "liquidity";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authority";
            type: "pubkey";
          },
          {
            name: "revenueCollector";
            type: "pubkey";
          },
          {
            name: "status";
            type: "bool";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logBorrowRateCap";
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logChangeStatus";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newStatus";
            type: "bool";
          }
        ];
      };
    },
    {
      name: "logClaim";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "recipient";
            type: "pubkey";
          },
          {
            name: "amount";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logCollectRevenue";
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "revenueAmount";
            type: "u128";
          }
        ];
      };
    },
    {
      name: "logOperate";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "supplyAmount";
            type: "i128";
          },
          {
            name: "borrowAmount";
            type: "i128";
          },
          {
            name: "withdrawTo";
            type: "pubkey";
          },
          {
            name: "borrowTo";
            type: "pubkey";
          },
          {
            name: "supplyExchangePrice";
            type: "u64";
          },
          {
            name: "borrowExchangePrice";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "logPauseUser";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logUnpauseUser";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "logUpdateAuthority";
      type: {
        kind: "struct";
        fields: [
          {
            name: "newAuthority";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateAuths";
      type: {
        kind: "struct";
        fields: [
          {
            name: "authStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateExchangePrices";
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "supplyExchangePrice";
            type: "u128";
          },
          {
            name: "borrowExchangePrice";
            type: "u128";
          },
          {
            name: "borrowRate";
            type: "u16";
          },
          {
            name: "utilization";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "logUpdateGuardians";
      type: {
        kind: "struct";
        fields: [
          {
            name: "guardianStatus";
            type: {
              vec: {
                defined: {
                  name: "addressBool";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateRateDataV1";
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "rateData";
            type: {
              defined: {
                name: "rateDataV1Params";
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateRateDataV2";
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "rateData";
            type: {
              defined: {
                name: "rateDataV2Params";
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateRevenueCollector";
      type: {
        kind: "struct";
        fields: [
          {
            name: "revenueCollector";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "logUpdateTokenConfigs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tokenConfig";
            type: {
              defined: {
                name: "tokenConfig";
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateUserBorrowConfigs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "userBorrowConfig";
            type: {
              defined: {
                name: "userBorrowConfig";
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateUserClass";
      type: {
        kind: "struct";
        fields: [
          {
            name: "userClass";
            type: {
              vec: {
                defined: {
                  name: "addressU8";
                };
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateUserSupplyConfigs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "userSupplyConfig";
            type: {
              defined: {
                name: "userSupplyConfig";
              };
            };
          }
        ];
      };
    },
    {
      name: "logUpdateUserWithdrawalLimit";
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "token";
            type: "pubkey";
          },
          {
            name: "newLimit";
            type: "u128";
          }
        ];
      };
    },
    {
      name: "rateDataV1Params";
      docs: ["@notice struct to set borrow rate data for version 1"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "kink";
            docs: [
              "",
              "@param kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationZero";
            docs: [
              "",
              "@param rateAtUtilizationZero desired borrow rate when utilization is zero. in 1e2: 100% = 10_000; 1% = 100",
              "i.e. constant minimum borrow rate",
              "e.g. at utilization = 0.01% rate could still be at least 4% (rateAtUtilizationZero would be 400 then)"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationKink";
            docs: [
              "",
              "@param rateAtUtilizationKink borrow rate when utilization is at kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at kink then rateAtUtilizationKink would be 700"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationMax";
            docs: [
              "",
              "@param rateAtUtilizationMax borrow rate when utilization is maximum at 100%. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 125% at 100% then rateAtUtilizationMax would be 12_500"
            ];
            type: "u128";
          }
        ];
      };
    },
    {
      name: "rateDataV2Params";
      docs: ["@notice struct to set borrow rate data for version 2"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "kink1";
            docs: [
              "",
              "@param kink1 first kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100",
              "utilization below kink 1 usually means slow increase in rate, once utilization is above kink 1 borrow rate increases faster"
            ];
            type: "u128";
          },
          {
            name: "kink2";
            docs: [
              "",
              "@param kink2 second kink in borrow rate. in 1e2: 100% = 10_000; 1% = 100",
              "utilization below kink 2 usually means slow / medium increase in rate, once utilization is above kink 2 borrow rate increases fast"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationZero";
            docs: [
              "",
              "@param rateAtUtilizationZero desired borrow rate when utilization is zero. in 1e2: 100% = 10_000; 1% = 100",
              "i.e. constant minimum borrow rate",
              "e.g. at utilization = 0.01% rate could still be at least 4% (rateAtUtilizationZero would be 400 then)"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationKink1";
            docs: [
              "",
              "@param rateAtUtilizationKink1 desired borrow rate when utilization is at first kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at first kink then rateAtUtilizationKink would be 700"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationKink2";
            docs: [
              "",
              "@param rateAtUtilizationKink2 desired borrow rate when utilization is at second kink. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 7% at second kink then rateAtUtilizationKink would be 1_200"
            ];
            type: "u128";
          },
          {
            name: "rateAtUtilizationMax";
            docs: [
              "",
              "@param rateAtUtilizationMax desired borrow rate when utilization is maximum at 100%. in 1e2: 100% = 10_000; 1% = 100",
              "e.g. when rate should be 125% at 100% then rateAtUtilizationMax would be 12_500"
            ];
            type: "u128";
          }
        ];
      };
    },
    {
      name: "rateModel";
      docs: ["Interest rate model data"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "version";
            type: "u8";
          },
          {
            name: "rateAtZero";
            type: "u16";
          },
          {
            name: "kink1Utilization";
            type: "u16";
          },
          {
            name: "rateAtKink1";
            type: "u16";
          },
          {
            name: "rateAtMax";
            type: "u16";
          },
          {
            name: "kink2Utilization";
            type: "u16";
          },
          {
            name: "rateAtKink2";
            type: "u16";
          }
        ];
      };
    },
    {
      name: "tokenConfig";
      docs: ["@notice struct to set token config"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "token";
            docs: ["", "@param token address"];
            type: "pubkey";
          },
          {
            name: "fee";
            docs: [
              "",
              "@param fee charges on borrower's interest. in 1e2: 100% = 10_000; 1% = 100"
            ];
            type: "u128";
          },
          {
            name: "maxUtilization";
            docs: [
              "",
              "@param maxUtilization maximum allowed utilization. in 1e2: 100% = 10_000; 1% = 100",
              "set to 100% to disable and have default limit of 100% (avoiding SLOAD)."
            ];
            type: "u128";
          }
        ];
      };
    },
    {
      name: "tokenReserve";
      docs: ["Token configuration and exchange prices"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "vault";
            type: "pubkey";
          },
          {
            name: "borrowRate";
            type: "u16";
          },
          {
            name: "feeOnInterest";
            type: "u16";
          },
          {
            name: "lastUtilization";
            type: "u16";
          },
          {
            name: "lastUpdateTimestamp";
            type: "u64";
          },
          {
            name: "supplyExchangePrice";
            type: "u64";
          },
          {
            name: "borrowExchangePrice";
            type: "u64";
          },
          {
            name: "maxUtilization";
            type: "u16";
          },
          {
            name: "totalSupplyWithInterest";
            type: "u64";
          },
          {
            name: "totalSupplyInterestFree";
            type: "u64";
          },
          {
            name: "totalBorrowWithInterest";
            type: "u64";
          },
          {
            name: "totalBorrowInterestFree";
            type: "u64";
          },
          {
            name: "totalClaimAmount";
            type: "u64";
          },
          {
            name: "interactingProtocol";
            type: "pubkey";
          },
          {
            name: "interactingTimestamp";
            type: "u64";
          },
          {
            name: "interactingBalance";
            type: "u64";
          }
        ];
      };
    },
    {
      name: "transferType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "skip";
          },
          {
            name: "direct";
          },
          {
            name: "claim";
          }
        ];
      };
    },
    {
      name: "userBorrowConfig";
      docs: ["@notice struct to set user borrow & payback config"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "mode";
            docs: ["", "@param mode: 0 = without interest. 1 = with interest"];
            type: "u8";
          },
          {
            name: "expandPercent";
            docs: [
              "",
              "@param expandPercent debt limit expand percent. in 1e2: 100% = 10_000; 1% = 100",
              "Also used to calculate rate at which debt limit should decrease (instant)."
            ];
            type: "u128";
          },
          {
            name: "expandDuration";
            docs: [
              "",
              "@param expandDuration debt limit expand duration in seconds.",
              "used to calculate rate together with expandPercent"
            ];
            type: "u128";
          },
          {
            name: "baseDebtCeiling";
            docs: [
              "",
              "@param baseDebtCeiling base borrow limit. until here, borrow limit remains as baseDebtCeiling",
              "(user can borrow until this point at once without stepped expansion). Above this, automated limit comes in place.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ];
            type: "u128";
          },
          {
            name: "maxDebtCeiling";
            docs: [
              "",
              "@param maxDebtCeiling max borrow ceiling, maximum amount the user can borrow.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ];
            type: "u128";
          }
        ];
      };
    },
    {
      name: "userBorrowPosition";
      docs: ["User borrow position"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "protocol";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "withInterest";
            type: "u8";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "debtCeiling";
            type: "u64";
          },
          {
            name: "lastUpdate";
            type: "u64";
          },
          {
            name: "expandPct";
            type: "u16";
          },
          {
            name: "expandDuration";
            type: "u32";
          },
          {
            name: "baseDebtCeiling";
            type: "u64";
          },
          {
            name: "maxDebtCeiling";
            type: "u64";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "userClaim";
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "user";
            type: "pubkey";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "mint";
            type: "pubkey";
          }
        ];
      };
    },
    {
      name: "userClass";
      type: {
        kind: "struct";
        fields: [
          {
            name: "addr";
            type: "pubkey";
          },
          {
            name: "class";
            type: "u8";
          }
        ];
      };
    },
    {
      name: "userSupplyConfig";
      docs: ["@notice struct to set user supply & withdrawal config"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "mode";
            docs: ["", "@param mode: 0 = without interest. 1 = with interest"];
            type: "u8";
          },
          {
            name: "expandPercent";
            docs: [
              "",
              "@param expandPercent withdrawal limit expand percent. in 1e2: 100% = 10_000; 1% = 100",
              "Also used to calculate rate at which withdrawal limit should decrease (instant)."
            ];
            type: "u128";
          },
          {
            name: "expandDuration";
            docs: [
              "",
              "@param expandDuration withdrawal limit expand duration in seconds.",
              "used to calculate rate together with expandPercent"
            ];
            type: "u128";
          },
          {
            name: "baseWithdrawalLimit";
            docs: [
              "",
              "@param baseWithdrawalLimit base limit, below this, user can withdraw the entire amount.",
              "amount in raw (to be multiplied with exchange price) or normal depends on configured mode in user config for the token:",
              "with interest -> raw, without interest -> normal"
            ];
            type: "u128";
          }
        ];
      };
    },
    {
      name: "userSupplyPosition";
      docs: ["User supply position"];
      serialization: "bytemuck";
      repr: {
        kind: "c";
        packed: true;
      };
      type: {
        kind: "struct";
        fields: [
          {
            name: "protocol";
            type: "pubkey";
          },
          {
            name: "mint";
            type: "pubkey";
          },
          {
            name: "withInterest";
            type: "u8";
          },
          {
            name: "amount";
            type: "u64";
          },
          {
            name: "withdrawalLimit";
            type: "u128";
          },
          {
            name: "lastUpdate";
            type: "u64";
          },
          {
            name: "expandPct";
            type: "u16";
          },
          {
            name: "expandDuration";
            type: "u64";
          },
          {
            name: "baseWithdrawalLimit";
            type: "u64";
          },
          {
            name: "status";
            type: "u8";
          }
        ];
      };
    }
  ];
};

```
---
## `docs/earn/api.md`

```markdown
# Jupiter Lend API Documentation

For the latest API Documentation, please refer to the following links:
- [Jupiter Lend API Documentation](https://dev.jup.ag/docs/lend-api)
- [Jupiter Lend API Schema](https://dev.jup.ag/docs/api/lend-api)
```
---
## `docs/earn/sdk.md`

```markdown
# Jupiter Lend Earn SDK Documentation

## Overview

The Jupiter Lend SDK provides a TypeScript interface for interacting with the Jupiter lending protocol. This documentation covers two main integration approaches: getting instruction objects for direct use and getting account contexts for Cross-Program Invocation (CPI) integrations.

## Installation

```bash
npm install @jup-ag/lend
```

## Setup

```typescript
import {
    Connection,
    Keypair, 
    PublicKey, 
    TransactionMessage, 
    TransactionInstruction, 
    VersionedTransaction
} from "@solana/web3.js";
import {
  getDepositIx, getWithdrawIx, // get instructions
  getDepositContext, getWithdrawContext, // get context accounts for CPI
} from "@jup-ag/lend/earn";
import { BN } from "bn.js";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const signer = Keypair.fromSecretKey(new Uint8Array(privateKey));

// Example asset mints
const usdc = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"); // USDC mainnet
```

---

## Instruction

### Get Deposit Instruction

```typescript
const depositIx = await getDepositIx({
    amount: new BN(1000000), // amount in token decimals (1 USDC)
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // asset mint address
    signer: signer.publicKey, // signer public key
    connection, // Solana connection
    cluster: "mainnet",
});
```

### Get Withdraw Instruction

```typescript
const withdrawIx = await getWithdrawIx({
    amount: new BN(1000000), // amount in token decimals (1 USDC)
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // asset mint address
    signer: signer.publicKey, // signer public key
    connection, // Solana connection
    cluster: "mainnet",
});
```

### Example Instruction Usage

```typescript
import {
    Connection,
    Keypair, 
    PublicKey, 
    TransactionMessage, 
    Transaction,
    TransactionInstruction,
    VersionedTransaction
} from "@solana/web3.js";
import {
    getDepositIx,
} from "@jup-ag/lend/earn";
import { BN } from "bn.js";

const signer = Keypair.fromSecretKey(new Uint8Array(privateKey));
const connection = new Connection('https://api.mainnet-beta.solana.com');

// Get deposit instruction
const depositIx = await getDepositIx({
    amount: new BN(1000000), // amount in token decimals (1 USDC)
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // asset mint address
    signer: signer.publicKey, // signer public key
    connection, // Solana connection
    cluster: "mainnet",
});

// Convert the raw instruction to TransactionInstruction
const instruction = new TransactionInstruction({
    programId: new PublicKey(depositIx.programId),
    keys: depositIx.keys.map((key) => ({
        pubkey: new PublicKey(key.pubkey),
        isSigner: key.isSigner,
        isWritable: key.isWritable,
    })),
    data: Buffer.from(depositIx.data),
});

const latestBlockhash = await connection.getLatestBlockhash();
const messageV0 = new TransactionMessage({
    payerKey: signer.publicKey,
    recentBlockhash: latestBlockhash.blockhash,
    instructions: [instruction],
}).compileToV0Message();

const transaction = new VersionedTransaction(messageV0);
transaction.sign([signer]);
const serializedTransaction = transaction.serialize();
const blockhashInfo = await connection.getLatestBlockhashAndContext({ commitment: "finalized" });

const signature = await connection.sendRawTransaction(serializedTransaction);
console.log(`https://solscan.io/tx/${signature}`);
```

## CPI

For Anchor programs that need to make CPI calls to Jupiter Lend, use the context methods.

### Deposit Context Accounts

```typescript
const depositContext = await getDepositContext({
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // asset mint address
    signer: signer.publicKey, // signer public key
    connection,
});
```

<details>
    <summary>
        <div>
            <div>
                <b>Deposit Context Accounts Table</b>
            </div>
        </div>
    </summary>

| Account                            | Purpose                                  |
| ---------------------------------- | ---------------------------------------- |
| `signer`                           | User's wallet public key                 |
| `depositorTokenAccount`            | User's underlying token account (source) |
| `recipientTokenAccount`            | User's fToken account (destination)      |
| `mint`                             | Underlying token mint                    |
| `lendingAdmin`                     | Protocol configuration PDA               |
| `lending`                          | Pool-specific configuration PDA          |
| `fTokenMint`                       | fToken mint account                      |
| `supplyTokenReservesLiquidity`     | Liquidity protocol token reserves        |
| `lendingSupplyPositionOnLiquidity` | Protocol's position in liquidity pool    |
| `rateModel`                        | Interest rate calculation model          |
| `vault`                            | Protocol vault holding deposited tokens  |
| `liquidity`                        | Main liquidity protocol PDA              |
| `liquidityProgram`                 | Liquidity protocol program ID            |
| `rewardsRateModel`                 | Rewards calculation model PDA            |
</details>

### Withdraw Context Accounts

```typescript
const withdrawContext = await getWithdrawContext({
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // asset mint address
    signer: signer.publicKey, // signer public key
    connection,
});
```

<details>
    <summary>
        <div>
            <div>
                <b>Withdraw Context Accounts Table</b>
            </div>
        </div>
    </summary>
Similar to deposit context, but includes:

- `ownerTokenAccount`: User's fToken account (source of fTokens to burn)
- `claimAccount`: Additional account for withdrawal claim processing

| Account                            | Purpose                                  |
| ---------------------------------- | ---------------------------------------- |
| `signer`                           | User's wallet public key                 |
| `ownerTokenAccount`                | User's underlying token account (source) |
| `recipientTokenAccount`            | User's fToken account (destination)      |
| `claimAccount`                     | Additional account for withdrawal        |
| `mint`                             | Underlying token mint                    |
| `lendingAdmin`                     | Protocol configuration PDA               | 
| `lending`                          | Pool-specific configuration PDA          |
| `fTokenMint`                       | fToken mint account                      |
| `supplyTokenReservesLiquidity`     | Liquidity protocol token reserves        |
| `lendingSupplyPositionOnLiquidity` | Protocol's position in liquidity pool    |
| `rateModel`                        | Interest rate calculation model          |
| `vault`                            | Protocol vault holding deposited tokens  |
| `liquidity`                        | Main liquidity protocol PDA              |
| `liquidityProgram`                 | Liquidity protocol program ID            |
| `rewardsRateModel`                 | Rewards calculation model PDA            |
</details>

### Example CPI Usage

```typescript
const depositContext = await getDepositContext({
  asset: usdcMint,
  signer: userPublicKey,
});

// Pass these accounts to your Anchor program
await program.methods
  .yourDepositMethod(amount)
  .accounts({
    // Your program accounts
    userAccount: userAccount,

    // Jupiter Lend accounts (from context)
    signer: depositContext.signer,
    depositorTokenAccount: depositContext.depositorTokenAccount,
    recipientTokenAccount: depositContext.recipientTokenAccount,
    lendingAdmin: depositContext.lendingAdmin,
    lending: depositContext.lending,
    fTokenMint: depositContext.fTokenMint,
    // ... all other accounts from context

    lendingProgram: new PublicKey(
      "7tjE28izRUjzmxC1QNXnNwcc4N82CNYCexf3k8mw67s3"
    ),
  })
  .rpc();
```

---

## Read Functions

The Jupiter Lend SDK provides several read functions to query protocol data and user positions, this can be helpful to display on your frontend.

### Get All Lending Tokens

Retrieves all available lending tokens in the Jupiter Lend Earn protocol.

The `getLendingTokens` function returns an array of `PublicKey` objects.

```typescript
import { getLendingTokens } from "@jup-ag/lend/earn";

const allTokens = await getLendingTokens({ connection });
```
```typescript
[
    PublicKey,
    PublicKey,
    ...
]
```

### Get Token Details

Fetches detailed information about a specific lending token.

```typescript
import { getLendingTokenDetails } from "@jup-ag/lend/earn";

const tokenDetails = await getLendingTokenDetails({
    lendingToken: new PublicKey("9BEcn9aPEmhSPbPQeFGjidRiEKki46fVQDyPpSQXPA2D"), // allTokens[x] from the previous example
    connection,
});
```
```typescript
{
  id: number; // ID of jlToken, starts from 1
  address: PublicKey; // Address of jlToken
  asset: PublicKey; // Address of underlying asset
  decimals: number; // Decimals of asset (same as jlToken decimals)
  totalAssets: BN; // Total underlying assets in the pool
  totalSupply: BN; // Total shares supply
  convertToShares: BN; // Multiplier to convert assets to shares
  convertToAssets: BN; // Multiplier to convert shares to assets
  rewardsRate: BN; // Rewards rate (1e4 decimals, 1e4 = 100%)
  supplyRate: BN; // Supply APY rate (1e4 decimals, 1e4 = 100%)
}
```

### Get User Position

Retrieves a user's lending position for a specific asset:

```typescript
import { getUserLendingPositionByAsset } from "@jup-ag/lend/earn";

const userPosition = await getUserLendingPositionByAsset({
    asset: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // The address of underlying asset or tokenDetails.asset
    user: signer.publicKey, // User's wallet address
    connection,
});
```
```typescript
{
  lendingTokenShares: BN; // User's shares in jlToken
  underlyingAssets: BN; // User's underlying assets
  underlyingBalance: BN; // User's underlying balance
}
```

```
---
## `docs/earn/cpi.md`

```markdown
# Jupiter Lend Earn CPI Documentation

## Overview

This documentation covers Cross-Program Invocation (CPI) integration for the lending protocol's core deposit and withdraw functionality using native Solana instructions. The protocol implements a vault-style system where users deposit underlying tokens and receive fTokens (share tokens) in return.

### Deployed address

#### Devnet

| Program           | Address                                        | link                                                                                                                |
| ----------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| LENDING_PROGRAM   | `7tjE28izRUjzmxC1QNXnNwcc4N82CNYCexf3k8mw67s3` | [lending_devnet](https://explorer.solana.com/address/7tjE28izRUjzmxC1QNXnNwcc4N82CNYCexf3k8mw67s3?cluster=devnet)   |
| LIQUIDITY_PROGRAM | `5uDkCoM96pwGYhAUucvCzLfm5UcjVRuxz6gH81RnRBmL` | [liquidity_devnet](https://explorer.solana.com/address/5uDkCoM96pwGYhAUucvCzLfm5UcjVRuxz6gH81RnRBmL?cluster=devnet) |

#### Staging mainnet

| Program           | Address                                       | link                                                                                                 |
| ----------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| LENDING_PROGRAM   | `jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9` | [lending_mainnet](https://explorer.solana.com/address/jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9)   |
| LIQUIDITY_PROGRAM | `jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC` | [liquidity_mainnet](https://explorer.solana.com/address/jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC) |

## Core CPI Functions

### 1. Deposit Flow

- `deposit` - Deposit assets, receive fTokens

### 2. Withdraw Flow

- `withdraw` - Withdraw assets by burning fTokens

---

## Deposit CPI Integration

### Function Discriminators

```rust
fn get_deposit_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:deposit")[0..8]
    vec![242, 35, 198, 137, 82, 225, 242, 182]
}
```

### Deposit CPI Struct

```rust
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub struct DepositParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub depositor_token_account: AccountInfo<'info>,
    pub recipient_token_account: AccountInfo<'info>,

    pub mint: AccountInfo<'info>,

    // Protocol accounts
    pub lending_admin: AccountInfo<'info>,
    pub lending: AccountInfo<'info>,
    pub f_token_mint: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub lending_supply_position_on_liquidity: AccountInfo<'info>,
    pub rate_model: AccountInfo<'info>,
    pub vault: AccountInfo<'info>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,

    // Rewards and programs
    pub rewards_rate_model: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,

    // Target lending program
    pub lending_program: UncheckedAccount<'info>,
}
```

### Deposit Implementation

```rust
impl<'info> DepositParams<'info> {
    pub fn deposit(&self, amount: u64) -> Result<()> {
        let mut instruction_data = get_deposit_discriminator();
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        let account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // depositor_token_account (mutable)
            AccountMeta::new(*self.depositor_token_account.key, false),
            // recipient_token_account (mutable)
            AccountMeta::new(*self.recipient_token_account.key, false),
            // mint
            AccountMeta::new_readonly(*self.mint.key, false),
            // lending_admin (readonly)
            AccountMeta::new_readonly(*self.lending_admin.key, false),
            // lending (mutable)
            AccountMeta::new(*self.lending.key, false),
            // f_token_mint (mutable)
            AccountMeta::new(*self.f_token_mint.key, false),
            // supply_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            // lending_supply_position_on_liquidity (mutable)
            AccountMeta::new(*self.lending_supply_position_on_liquidity.key, false),
            // rate_model (readonly)
            AccountMeta::new_readonly(*self.rate_model.key, false),
            // vault (mutable)
            AccountMeta::new(*self.vault.key, false),
            // liquidity (mutable)
            AccountMeta::new(*self.liquidity.key, false),
            // liquidity_program (mutable)
            AccountMeta::new(*self.liquidity_program.key, false),
            // rewards_rate_model (readonly)
            AccountMeta::new_readonly(*self.rewards_rate_model.key, false),
            // token_program
            AccountMeta::new_readonly(*self.token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.lending_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.depositor_token_account.clone(),
                self.recipient_token_account.clone(),
                self.mint.clone(),
                self.lending_admin.clone(),
                self.lending.clone(),
                self.f_token_mint.clone(),
                self.supply_token_reserves_liquidity.clone(),
                self.lending_supply_position_on_liquidity.clone(),
                self.rate_model.clone(),
                self.vault.clone(),
                self.liquidity.clone(),
                self.liquidity_program.clone(),
                self.rewards_rate_model.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| ErrorCodes::CpiToLendingProgramFailed.into())
    }
}
```

> Full snippet available [here](../../references/earn/deposit.rs)

### Deposit Account Explanations

| Account                                | Purpose                         | Mutability | Notes                                 |
| -------------------------------------- | ------------------------------- | ---------- | ------------------------------------- |
| `signer`                               | User performing deposit         | Mutable    | Signs the transaction                 |
| `depositor_token_account`              | User's underlying token account | Mutable    | Source of tokens to deposit           |
| `recipient_token_account`              | User's fToken account           | Mutable    | Destination for minted fTokens        |
| `mint`                                 | Underlying token mint           | Immutable  | The token being deposited             |
| `lending_admin`                        | Protocol configuration          | Immutable  | Contains liquidity program reference  |
| `lending`                              | Pool-specific configuration     | Mutable    | Links mint to fToken mint             |
| `f_token_mint`                         | fToken mint account             | Mutable    | fTokens minted to supply              |
| `supply_token_reserves_liquidity`      | Liquidity reserves              | Mutable    | Liquidity protocol token reserves     |
| `lending_supply_position_on_liquidity` | Lending position                | Mutable    | Protocol's position in liquidity pool |
| `rate_model`                           | Interest rate calculation       | Immutable  | Determines interest rates             |
| `vault`                                | Protocol token vault            | Mutable    | Destination of deposited tokens       |
| `liquidity`                            | Liquidity protocol PDA          | Mutable    | Manages liquidity operations          |
| `liquidity_program`                    | Liquidity program reference     | Mutable    | External liquidity program            |
| `rewards_rate_model`                   | Rewards calculation             | Immutable  | Determines fToken exchange rate       |

---

## Withdraw CPI Integration

### Function Discriminators

```rust
fn get_withdraw_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:withdraw")[0..8]
    vec![183, 18, 70, 156, 148, 109, 161, 34]
}
```

### Withdraw CPI Struct

```rust
pub struct WithdrawParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub owner_token_account: AccountInfo<'info>,
    pub recipient_token_account: AccountInfo<'info>,

    // Protocol accounts
    pub lending_admin: AccountInfo<'info>,
    pub lending: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub f_token_mint: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub lending_supply_position_on_liquidity: AccountInfo<'info>,
    pub rate_model: AccountInfo<'info>,
    pub vault: AccountInfo<'info>,
    pub claim_account: AccountInfo<'info>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,

    // Rewards and programs
    pub rewards_rate_model: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,

    // Target lending program
    pub lending_program: UncheckedAccount<'info>,
}
```

### Withdraw Implementation

```rust
impl<'info> WithdrawParams<'info> {
    pub fn withdraw(&self, assets: u64) -> Result<()> {
        let mut instruction_data = get_withdraw_discriminator();
        instruction_data.extend_from_slice(&assets.to_le_bytes());

        let account_metas = vec![
            // signer (mutable, signer)
            AccountMeta::new(*self.signer.key, true),
            // owner_token_account (mutable) - user's fToken account
            AccountMeta::new(*self.owner_token_account.key, false),
            // recipient_token_account (mutable) - user's underlying token account
            AccountMeta::new(*self.recipient_token_account.key, false),
            // lending_admin (readonly)
            AccountMeta::new_readonly(*self.lending_admin.key, false),
            // lending (mutable)
            AccountMeta::new(*self.lending.key, false),
            // mint (readonly) - underlying token mint
            AccountMeta::new_readonly(*self.mint.key, false),
            // f_token_mint (mutable)
            AccountMeta::new(*self.f_token_mint.key, false),
            // supply_token_reserves_liquidity (mutable)
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            // lending_supply_position_on_liquidity (mutable)
            AccountMeta::new(*self.lending_supply_position_on_liquidity.key, false),
            // rate_model (readonly)
            AccountMeta::new_readonly(*self.rate_model.key, false),
            // vault (mutable)
            AccountMeta::new(*self.vault.key, false),
            // claim_account (mutable)
            AccountMeta::new(*self.claim_account.key, false),
            // liquidity (mutable)
            AccountMeta::new(*self.liquidity.key, false),
            // liquidity_program (mutable)
            AccountMeta::new(*self.liquidity_program.key, false),
            // rewards_rate_model (readonly)
            AccountMeta::new_readonly(*self.rewards_rate_model.key, false),
            // token_program
            AccountMeta::new_readonly(*self.token_program.key, false),
            // associated_token_program
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            // system_program
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.lending_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.owner_token_account.clone(),
                self.recipient_token_account.clone(),
                self.lending_admin.clone(),
                self.lending.clone(),
                self.mint.clone(),
                self.f_token_mint.clone(),
                self.supply_token_reserves_liquidity.clone(),
                self.lending_supply_position_on_liquidity.clone(),
                self.rate_model.clone(),
                self.vault.clone(),
                self.claim_account.clone(),
                self.liquidity.clone(),
                self.liquidity_program.clone(),
                self.rewards_rate_model.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| ErrorCodes::CpiToLendingProgramFailed.into())
    }
}
```

> Full snippet available [here](../../references/earn/withdraw.rs)

### Withdraw Account Explanations

| Account                                | Purpose                         | Mutability | Notes                                 |
| -------------------------------------- | ------------------------------- | ---------- | ------------------------------------- |
| `signer`                               | User performing withdrawal      | Mutable    | Must own fTokens to burn              |
| `owner_token_account`                  | User's fToken account           | Mutable    | Source of fTokens to burn             |
| `recipient_token_account`              | User's underlying token account | Mutable    | Destination for withdrawn tokens      |
| `lending_admin`                        | Protocol configuration          | Immutable  | Contains liquidity program reference  |
| `lending`                              | Pool-specific configuration     | Mutable    | Links mint to fToken mint             |
| `mint`                                 | Underlying token mint           | Immutable  | The token being withdrawn             |
| `f_token_mint`                         | fToken mint account             | Mutable    | fTokens burned from supply            |
| `supply_token_reserves_liquidity`      | Liquidity reserves              | Mutable    | Liquidity protocol token reserves     |
| `lending_supply_position_on_liquidity` | Lending position                | Mutable    | Protocol's position in liquidity pool |
| `rate_model`                           | Interest rate calculation       | Immutable  | Determines interest rates             |
| `vault`                                | Protocol token vault            | Mutable    | Source of withdrawn tokens            |
| `claim_account`                        | Claim processing account        | Mutable    | Handles withdrawal claims             |
| `liquidity`                            | Liquidity protocol PDA          | Mutable    | Manages liquidity operations          |
| `liquidity_program`                    | Liquidity program reference     | Mutable    | External liquidity program            |
| `rewards_rate_model`                   | Rewards calculation             | Immutable  | Determines fToken exchange rate       |

---

## Key Implementation Notes

### 1. Account Derivation

Most accounts follow standard PDA derivation patterns:

- Lending PDA: `[LENDING_SEED, mint.key(), f_token_mint.key()]`
- fToken Mint: `[F_TOKEN_MINT_SEED, mint.key()]`
- Lending Admin: `[LENDING_ADMIN_SEED]`

### 2. Special Considerations

- **Amount = u64::MAX**: Deposits/withdraws the entire balance
- **Account Creation**: ATA accounts are created automatically when needed (`init_if_needed`)
- **Liquidity Integration**: The protocol integrates with an underlying liquidity protocol
- **Claim Account**: Only present in withdraw operations for processing withdrawal claims

### 3. Error Handling

Common errors to handle:

- `FTokenMinAmountOut`: Slippage protection triggered
- `FTokenMaxAmount`: Maximum amount exceeded
- `FTokenOnlyAuth`: Unauthorized operation
- `FTokenOnlyRebalancer`: Rebalancer-only operation
- `CpiToLendingProgramFailed`: CPI call failed

### 4. Return Values

- `deposit()` returns shares minted
- `withdraw()` returns shares burned

---

```
---
## `docs/borrow/sdk.md`

```markdown
# Jupiter Vaults SDK Documentation

## Overview

The Jupiter Vaults SDK provides a TypeScript interface for interacting with the Jupiter Vaults protocol. This documentation covers the main integration approach: getting instruction objects and account contexts for vault operations including deposit, withdraw, borrow, and payback through a single `operate` function.

## Installation

```bash
npm install @jup-ag/lend
```

## Setup

```typescript
import { Connection, PublicKey, Transaction } from "@solana/web3.js";
import { getOperateIx } from "@jup-ag/lend/borrow";
import { BN } from "bn.js";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const signer = new PublicKey("YOUR_SIGNER_PUBLIC_KEY");

// Example vault configuration
const vaultId = 1; // Your vault ID
const positionId = 12345; // Your position NFT ID (obtained after minting position NFT)
```

---

## Core Operation Function

### Getting Operate Instruction

Use `getOperateIx()` to get transaction instructions and all necessary account data for vault operations. The function returns multiple instructions that must be executed in order using **v0 (versioned) transactions**:

```typescript
// Get operate instruction with all accounts and data
const {
  ixs,
  addressLookupTableAccounts,
  nftId,
  accounts,
  remainingAccounts,
  remainingAccountsIndices,
} = await getOperateIx({
  colAmount: new BN(1000000000), // Collateral amount (1000 tokens scaled to 1e9)
  debtAmount: new BN(500000000), // Debt amount (500 tokens scaled to 1e9)
  connection,
  positionId: nftId, // Position NFT ID (to create a new position pass it as 0)
  signer: publicKey, // Signer public key
  vaultId: vault_id, // Vault ID
  cluster: "mainnet",
});

// IMPORTANT: Must use v0 (versioned) transaction
const latestBlockhash = await connection.getLatestBlockhash();

// Create transaction message with all instructions in order
const messageV0 = new TransactionMessage({
  payerKey: signer,
  recentBlockhash: latestBlockhash.blockhash,
  instructions: ixs, // All instructions must be added in order
}).compileToV0Message(addressLookupTableAccounts); // Include lookup table accounts

// Create versioned transaction
const versionedTransaction = new VersionedTransaction(messageV0);

// Sign and send versioned transaction
versionedTransaction.sign([signerKeypair]);
const signature = await connection.sendTransaction(versionedTransaction);
console.log("Transaction ID:", signature);
```

### Automatic Position Creation

If `positionId = 0` is provided, the function will automatically batch position creation instructions:

```typescript
// Create new position and perform operation in one transaction
const { ixs, addressLookupTableAccounts, nftId } = await getOperateIx({
  colAmount: new BN(1000000000),
  debtAmount: new BN(0),
  connection,
  positionId: 0, // No position ID = auto-create position
  signer: publicKey,
  vaultId: 1,
  cluster: "mainnet",
});

console.log("New position NFT ID:", nftId); // ID of the created position
console.log("Instructions count:", ixs.length); // Will include position creation + setup + operate

// Must use v0 transaction with lookup tables
const latestBlockhash = await connection.getLatestBlockhash();
const messageV0 = new TransactionMessage({
  payerKey: signer,
  recentBlockhash: latestBlockhash.blockhash,
  instructions: ixs,
}).compileToV0Message(addressLookupTableAccounts);

const versionedTransaction = new VersionedTransaction(messageV0);
versionedTransaction.sign([signerKeypair]);

const signature = await connection.sendTransaction(versionedTransaction);
```

---

## Operation Types

### 1. Deposit Only

```typescript
// Deposit 1000 supply tokens (with automatic position creation if needed)
const { ixs, nftId } = await getOperateIx({
  colAmount: new BN(1000000000), // Positive = deposit
  debtAmount: new BN(0), // No debt change
  connection,
  positionId: 0, // Will create new position automatically
  signer: publicKey,
  vaultId: 1,
  cluster: "mainnet",
});

console.log("Position NFT ID:", nftId); // Will be the new or existing position ID
```

### 2. Withdraw Only

```typescript
// Withdraw 500 supply tokens
const { ixs } = await getOperateIx({
  colAmount: new BN(-500000000), // Negative = withdraw
  debtAmount: new BN(0), // No debt change
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 3. Borrow Only

```typescript
// Borrow 250 borrow tokens
const { ixs } = await getOperateIx({
  colAmount: new BN(0), // No collateral change
  debtAmount: new BN(250000000), // Positive = borrow
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 4. Payback Only

```typescript
// Payback 100 borrow tokens
const { ixs } = await getOperateIx({
  colAmount: new BN(0), // No collateral change
  debtAmount: new BN(-100000000), // Negative = payback
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 5. Deposit + Borrow (Leverage)

```typescript
// Deposit 1000 tokens and borrow 400 tokens
const { ixs } = await getOperateIx({
  colAmount: new BN(1000000000), // Deposit collateral
  debtAmount: new BN(400000000), // Borrow debt
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 6. Payback + Withdraw (Deleverage)

```typescript
// Payback 200 tokens and withdraw 300 tokens
const { ixs } = await getOperateIx({
  colAmount: new BN(-300000000), // Withdraw collateral
  debtAmount: new BN(-200000000), // Payback debt
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 7. Max Withdraw

```typescript
// Withdraw all available collateral
const { ixs } = await getOperateIx({
  colAmount: new BN("-170141183460469231731687303715884105728"), // i128::MIN for max withdraw
  debtAmount: new BN(0),
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

### 8. Max Payback

```typescript
// Payback all debt
const { ixs } = await getOperateIx({
  colAmount: new BN(0),
  debtAmount: new BN("-170141183460469231731687303715884105728"), // i128::MIN for max payback
  connection,
  positionId: nft.id,
  signer: publicKey,
  vaultId: nft.vault.id,
  cluster: "mainnet",
});
```

---

## Return Object Properties

The `getOperateIx()` function returns an object with the following properties:

```typescript
interface OperateIxResponse {
  ixs: TransactionInstruction[]; // Array of transaction instructions
  addressLookupTableAccounts: AddressLookupTableAccount[]; // Lookup table accounts for optimization
  nftId: number; // Position NFT ID
  accounts: OperateAccounts; // All account addresses used in the operation
  remainingAccounts: PublicKey[]; // Additional accounts (oracle sources, branches, ticks)
  remainingAccountsIndices: number[]; // Indices for remaining accounts categorization
}

interface OperateAccounts {
  signer: PublicKey;
  signerSupplyTokenAccount: PublicKey;
  signerBorrowTokenAccount: PublicKey;
  recipient: PublicKey;
  recipientBorrowTokenAccount: PublicKey;
  recipientSupplyTokenAccount: PublicKey;
  vaultConfig: PublicKey;
  vaultState: PublicKey;
  supplyToken: PublicKey;
  borrowToken: PublicKey;
  oracle: PublicKey;
  position: PublicKey;
  positionTokenAccount: PublicKey;
  currentPositionTick: PublicKey;
  finalPositionTick: PublicKey;
  currentPositionTickId: PublicKey;
  finalPositionTickId: PublicKey;
  newBranch: PublicKey;
  supplyTokenReservesLiquidity: PublicKey;
  borrowTokenReservesLiquidity: PublicKey;
  vaultSupplyPositionOnLiquidity: PublicKey;
  vaultBorrowPositionOnLiquidity: PublicKey;
  supplyRateModel: PublicKey;
  borrowRateModel: PublicKey;
  vaultSupplyTokenAccount: PublicKey;
  vaultBorrowTokenAccount: PublicKey;
  supplyTokenClaimAccount?: PublicKey; // Optional for claim operations
  borrowTokenClaimAccount?: PublicKey; // Optional for claim operations
  liquidity: PublicKey;
  liquidityProgram: PublicKey;
  oracleProgram: PublicKey;
  supplyTokenProgram: PublicKey;
  borrowTokenProgram: PublicKey;
  associatedTokenProgram: PublicKey;
  systemProgram: PublicKey;
}
```

---

## CPI Integration Usage

For Anchor programs that need to make CPI calls to Jupiter Vaults, you need to handle the setup instructions separately from the final operate instruction:

```typescript
// In your frontend/client code
const { ixs, accounts, remainingAccounts, remainingAccountsIndices } =
  await getOperateIx({
    colAmount: new BN(1000000000),
    debtAmount: new BN(500000000),
    connection,
    positionId: nft.id,
    signer: userPublicKey,
    vaultId: nft.vault.id,
    cluster: "mainnet",
  });

// IMPORTANT: For CPI integration, you need to:
// 1. Execute setup instructions (all except the last one) in your transaction
// 2. Use the last instruction's accounts for your CPI call

// Setup instructions (all except last) - these prepare the environment
const setupInstructions = ixs.slice(0, -1); // Remove last instruction
const operateInstruction = ixs[ixs.length - 1]; // Last instruction is the actual direct operate call, for CPIs not needed

// Your transaction should include setup instructions first using v0 transaction
const latestBlockhash = await connection.getLatestBlockhash();

const messageV0 = new TransactionMessage({
  payerKey: userPublicKey,
  recentBlockhash: latestBlockhash.blockhash,
  instructions: [...setupInstructions /* your program instruction here */],
}).compileToV0Message(addressLookupTableAccounts);

const versionedTx = new VersionedTransaction(messageV0);

// Then your program instruction that makes CPI call
await program.methods
  .yourVaultOperateMethod(colAmount, debtAmount, remainingAccountsIndices)
  .accounts({
    // Your program accounts
    userAccount: userAccount,

    // Jupiter Vaults accounts (from context) - use accounts from the operate instruction
    signer: accounts.signer,
    signerSupplyTokenAccount: accounts.signerSupplyTokenAccount,
    signerBorrowTokenAccount: accounts.signerBorrowTokenAccount,
    recipient: accounts.recipient,
    recipientBorrowTokenAccount: accounts.recipientBorrowTokenAccount,
    recipientSupplyTokenAccount: accounts.recipientSupplyTokenAccount,
    vaultConfig: accounts.vaultConfig,
    vaultState: accounts.vaultState,
    supplyToken: accounts.supplyToken,
    borrowToken: accounts.borrowToken,
    oracle: accounts.oracle,
    position: accounts.position,
    positionTokenAccount: accounts.positionTokenAccount,
    currentPositionTick: accounts.currentPositionTick,
    finalPositionTick: accounts.finalPositionTick,
    currentPositionTickId: accounts.currentPositionTickId,
    finalPositionTickId: accounts.finalPositionTickId,
    newBranch: accounts.newBranch,
    supplyTokenReservesLiquidity: accounts.supplyTokenReservesLiquidity,
    borrowTokenReservesLiquidity: accounts.borrowTokenReservesLiquidity,
    vaultSupplyPositionOnLiquidity: accounts.vaultSupplyPositionOnLiquidity,
    vaultBorrowPositionOnLiquidity: accounts.vaultBorrowPositionOnLiquidity,
    supplyRateModel: accounts.supplyRateModel,
    borrowRateModel: accounts.borrowRateModel,
    vaultSupplyTokenAccount: accounts.vaultSupplyTokenAccount,
    vaultBorrowTokenAccount: accounts.vaultBorrowTokenAccount,
    liquidity: accounts.liquidity,
    liquidityProgram: accounts.liquidityProgram,
    oracleProgram: accounts.oracleProgram,
    supplyTokenProgram: accounts.supplyTokenProgram,
    borrowTokenProgram: accounts.borrowTokenProgram,
    associatedTokenProgram: accounts.associatedTokenProgram,
    systemProgram: accounts.systemProgram,

    vaultsProgram: new PublicKey(
      "Ho32sUQ4NzuAQgkPkHuNDG3G18rgHmYtXFA8EBmqQrAu"
    ), // Devnet
  })
  .remainingAccounts(remainingAccounts)
  .rpc();
```

### CPI Setup Instructions

The setup instructions handle:

- Account initialization (if needed)
- Token account creation
- Tick and branch setup

**Important**: These setup instructions must be executed before your CPI call, as they prepare the program state for the vault operation.

---

## Account Explanations

### Core Vault Accounts

| Account                       | Purpose                                           |
| ----------------------------- | ------------------------------------------------- |
| `signer`                      | User's wallet public key performing the operation |
| `signerSupplyTokenAccount`    | User's supply token account (source for deposits) |
| `signerBorrowTokenAccount`    | User's borrow token account (source for paybacks) |
| `recipient`                   | Destination wallet for withdrawals/borrows        |
| `recipientSupplyTokenAccount` | Destination for withdrawn supply tokens           |
| `recipientBorrowTokenAccount` | Destination for borrowed tokens                   |

### Vault Configuration

| Account       | Purpose                                       |
| ------------- | --------------------------------------------- |
| `vaultConfig` | Vault configuration PDA containing parameters |
| `vaultState`  | Vault state PDA with current liquidity data   |
| `supplyToken` | Supply token mint address                     |
| `borrowToken` | Borrow token mint address                     |
| `oracle`      | Price oracle account for the vault            |

### Position Management

| Account                 | Purpose                                             |
| ----------------------- | --------------------------------------------------- |
| `position`              | User's position PDA containing debt/collateral data |
| `positionTokenAccount`  | User's position NFT token account                   |
| `currentPositionTick`   | Current tick where position is located              |
| `finalPositionTick`     | Final tick after operation                          |
| `currentPositionTickId` | Current position ID within tick                     |
| `finalPositionTickId`   | Final position ID within tick                       |
| `newBranch`             | Branch account for tick organization                |

### Liquidity Integration

| Account                          | Purpose                                       |
| -------------------------------- | --------------------------------------------- |
| `supplyTokenReservesLiquidity`   | Underlying liquidity protocol supply reserves |
| `borrowTokenReservesLiquidity`   | Underlying liquidity protocol borrow reserves |
| `vaultSupplyPositionOnLiquidity` | Vault's supply position in liquidity protocol |
| `vaultBorrowPositionOnLiquidity` | Vault's borrow position in liquidity protocol |
| `supplyRateModel`                | Supply interest rate model                    |
| `borrowRateModel`                | Borrow interest rate model                    |
| `vaultSupplyTokenAccount`        | Vault's supply token holding account          |
| `vaultBorrowTokenAccount`        | Vault's borrow token holding account          |
| `liquidity`                      | Main liquidity protocol PDA                   |
| `liquidityProgram`               | Liquidity protocol program ID                 |

### Remaining Accounts Structure

The `remainingAccountsIndices` array contains three values:

- `[0]` = Number of oracle source accounts
- `[1]` = Number of branch accounts
- `[2]` = Number of tick has debt array accounts

The `remainingAccounts` array is ordered as:

1. Oracle sources (0 to indices[0])
2. Branch accounts (indices[0] to indices[0] + indices[1])
3. Tick has debt arrays (indices[0] + indices[1] to indices[0] + indices[1] + indices[2])

---

## Important Notes

### Amount Scaling

- All amounts are scaled to 1e9 decimals internally by the vault
- Use `new BN('number')` for amounts to handle large numbers
- Positive values = deposit/borrow, Negative values = withdraw/payback
- Use `new BN('-170141183460469231731687303715884105728')` for max withdraw/payback operations

### Position Requirements

- Position NFT can be created automatically by passing `positionId = 0` parameter
- If `positionId` is provided, it will use the existing position
- Position NFT ownership is required for withdraw/borrow operations
- Anyone can deposit to any position or payback debt for any position

### Instructions Batching

- The `ixs` array contains multiple instructions that must be executed in order
- Instructions include: setup, account creation, environment preparation, and the final operate call
- All instructions are required for proper vault operation
- For CPI integration, execute setup instructions first, then make your CPI call with the operate instruction accounts

### Transaction Requirements

- **Must use v0 (versioned) transactions** - Regular transactions are not supported
- Address lookup tables are always provided and must be included in the transaction
- Multiple instructions are returned and must be executed in order
- For CPI integration, execute setup instructions first, then make your CPI call with the operate instruction accounts

### Error Handling

Common errors to handle:

- Invalid position ID or vault ID
- Insufficient collateral for borrow operations
- Position liquidation state conflicts
- Network connectivity issues

---

## Position NFT Creation

Position NFTs are automatically created when `positionId` is not provided:

```typescript
// Create new position and deposit in one transaction
const { ixs, nftId, accounts } = await getOperateIx({
  colAmount: new BN(1000000000),
  debtAmount: new BN(0),
  connection,
  positionId: 0,
  signer: publicKey,
  vaultId: 1,
  cluster: "mainnet",
});

console.log("Created position NFT ID:", nftId);

// Use existing position for subsequent operations
const { ixs: subsequentIxs } = await getOperateIx({
  colAmount: new BN(500000000),
  debtAmount: new BN(200000000),
  connection,
  positionId: nftId, // Use the created position
  signer: publicKey,
  vaultId: 1,
  cluster: "mainnet",
});
```

```
---
## `docs/borrow/cpi.md`

```markdown
# Jupiter Vaults CPI Documentation

## Overview

This documentation covers Cross-Program Invocation (CPI) integration for Jupiter Vaults, a sophisticated lending and borrowing protocol. The vault system uses NFT-based positions to manage user collateral and debt, with operations handled through a single `operate` function after initial position setup.

### Deployed Addresses

#### Devnet

| Program        | Address                                        | Link                                                                                                             |
| -------------- | ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| VAULTS_PROGRAM | `Ho32sUQ4NzuAQgkPkHuNDG3G18rgHmYtXFA8EBmqQrAu` | [vaults_devnet](https://explorer.solana.com/address/Ho32sUQ4NzuAQgkPkHuNDG3G18rgHmYtXFA8EBmqQrAu?cluster=devnet) |

#### Staging Mainnet

| Program        | Address                                       | Link                                                                                              |
| -------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| VAULTS_PROGRAM | `jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi` | [vaults_mainnet](https://explorer.solana.com/address/jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi) |

## Core Operation Flow

### Prerequisites

1. **Initialize Position NFT** - Required before any vault operations
2. **Operate** - Single function for all deposit/withdraw/borrow/payback operations

### Operation Types

- **Deposit + Borrow** - Supply collateral and borrow against it
- **Payback + Withdraw** - Repay debt and withdraw collateral

---

## 1. Initialize Position NFT

### Function Discriminator

```rust
fn get_init_position_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:init_position")[0..8]
    vec![197, 20, 10, 1, 97, 160, 177, 91]
}
```

### Init Position CPI Struct

```rust
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub struct InitPositionParams<'info> {
    pub signer: AccountInfo<'info>,
    pub vault_admin: AccountInfo<'info>,
    pub vault_state: AccountInfo<'info>,
    pub position: AccountInfo<'info>,
    pub position_mint: AccountInfo<'info>,
    pub position_token_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub vaults_program: UncheckedAccount<'info>,
}
```

### Init Position Implementation

```rust
impl<'info> InitPositionParams<'info> {
    pub fn init_position(&self, vault_id: u16, position_id: u32) -> Result<()> {
        let mut instruction_data = get_init_position_discriminator();
        instruction_data.extend_from_slice(&vault_id.to_le_bytes());
        instruction_data.extend_from_slice(&position_id.to_le_bytes());

        let account_metas = vec![
            AccountMeta::new(*self.signer.key, true),
            AccountMeta::new(*self.vault_admin.key, false),
            AccountMeta::new(*self.vault_state.key, false),
            AccountMeta::new(*self.position.key, false),
            AccountMeta::new(*self.position_mint.key, false),
            AccountMeta::new(*self.position_token_account.key, false),
            AccountMeta::new_readonly(*self.token_program.key, false),
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            AccountMeta::new_readonly(*self.system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *self.vaults_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &instruction,
            &[
                self.signer.clone(),
                self.vault_admin.clone(),
                self.vault_state.clone(),
                self.position.clone(),
                self.position_mint.clone(),
                self.position_token_account.clone(),
                self.token_program.clone(),
                self.associated_token_program.clone(),
                self.system_program.clone(),
            ],
        )
        .map_err(|_| ErrorCodes::CpiToVaultsProgramFailed.into())
    }
}
```

---

## 2. Operate Function (Deposit/Withdraw/Borrow/Payback)

### Function Discriminator

```rust
fn get_operate_discriminator() -> Vec<u8> {
    // discriminator = sha256("global:operate")[0..8]
    vec![217, 106, 208, 99, 116, 151, 42, 135]
}
```

### Operate CPI Struct

```rust
pub struct OperateParams<'info> {
    // User accounts
    pub signer: AccountInfo<'info>,
    pub signer_supply_token_account: AccountInfo<'info>,
    pub signer_borrow_token_account: AccountInfo<'info>,
    pub recipient: AccountInfo<'info>,
    pub recipient_borrow_token_account: AccountInfo<'info>,
    pub recipient_supply_token_account: AccountInfo<'info>,

    // Vault accounts
    pub vault_config: AccountInfo<'info>,
    pub vault_state: AccountInfo<'info>,
    pub supply_token: AccountInfo<'info>,
    pub borrow_token: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,

    // Position accounts
    pub position: AccountInfo<'info>,
    pub position_token_account: AccountInfo<'info>,
    pub current_position_tick: AccountInfo<'info>,
    pub final_position_tick: AccountInfo<'info>,
    pub current_position_tick_id: AccountInfo<'info>,
    pub final_position_tick_id: AccountInfo<'info>,
    pub new_branch: AccountInfo<'info>,

    // Liquidity protocol accounts
    pub supply_token_reserves_liquidity: AccountInfo<'info>,
    pub borrow_token_reserves_liquidity: AccountInfo<'info>,
    pub vault_supply_position_on_liquidity: AccountInfo<'info>,
    pub vault_borrow_position_on_liquidity: AccountInfo<'info>,
    pub supply_rate_model: AccountInfo<'info>,
    pub borrow_rate_model: AccountInfo<'info>,
    pub vault_supply_token_account: AccountInfo<'info>,
    pub vault_borrow_token_account: AccountInfo<'info>,
    pub supply_token_claim_account: Option<AccountInfo<'info>>,
    pub borrow_token_claim_account: Option<AccountInfo<'info>>,
    pub liquidity: AccountInfo<'info>,
    pub liquidity_program: AccountInfo<'info>,
    pub oracle_program: AccountInfo<'info>,

    // Programs
    pub supply_token_program: AccountInfo<'info>,
    pub borrow_token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub vaults_program: UncheckedAccount<'info>,
}
```

### Operate Implementation

```rust
impl<'info> OperateParams<'info> {
    pub fn operate(
        &self,
        new_col: i128,
        new_debt: i128,
        transfer_type: Option<u8>, // 0 = Normal, 1 = Claim
        remaining_accounts_indices: Vec<u8>,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<(u32, i128, i128)> {
        let mut instruction_data = get_operate_discriminator();
        instruction_data.extend_from_slice(&new_col.to_le_bytes());
        instruction_data.extend_from_slice(&new_debt.to_le_bytes());

        // Serialize transfer_type
        match transfer_type {
            Some(t) => {
                instruction_data.push(1); // Some
                instruction_data.push(t);
            },
            None => instruction_data.push(0), // None
        }

        // Serialize remaining_accounts_indices
        instruction_data.push(remaining_accounts_indices.len() as u8);
        instruction_data.extend_from_slice(&remaining_accounts_indices);

        let mut account_metas = vec![
            AccountMeta::new(*self.signer.key, true),
            AccountMeta::new(*self.signer_supply_token_account.key, false),
            AccountMeta::new(*self.signer_borrow_token_account.key, false),
            AccountMeta::new_readonly(*self.recipient.key, false),
            AccountMeta::new(*self.recipient_borrow_token_account.key, false),
            AccountMeta::new(*self.recipient_supply_token_account.key, false),
            AccountMeta::new(*self.vault_config.key, false),
            AccountMeta::new(*self.vault_state.key, false),
            AccountMeta::new_readonly(*self.supply_token.key, false),
            AccountMeta::new_readonly(*self.borrow_token.key, false),
            AccountMeta::new_readonly(*self.oracle.key, false),
            AccountMeta::new(*self.position.key, false),
            AccountMeta::new_readonly(*self.position_token_account.key, false),
            AccountMeta::new(*self.current_position_tick.key, false),
            AccountMeta::new(*self.final_position_tick.key, false),
            AccountMeta::new(*self.current_position_tick_id.key, false),
            AccountMeta::new(*self.final_position_tick_id.key, false),
            AccountMeta::new(*self.new_branch.key, false),
            AccountMeta::new(*self.supply_token_reserves_liquidity.key, false),
            AccountMeta::new(*self.borrow_token_reserves_liquidity.key, false),
            AccountMeta::new(*self.vault_supply_position_on_liquidity.key, false),
            AccountMeta::new(*self.vault_borrow_position_on_liquidity.key, false),
            AccountMeta::new(*self.supply_rate_model.key, false),
            AccountMeta::new(*self.borrow_rate_model.key, false),
            AccountMeta::new(*self.vault_supply_token_account.key, false),
            AccountMeta::new(*self.vault_borrow_token_account.key, false),
        ];

        // Add optional claim accounts
        if let Some(ref claim_account) = self.supply_token_claim_account {
            account_metas.push(AccountMeta::new(*claim_account.key, false));
        }
        if let Some(ref claim_account) = self.borrow_token_claim_account {
            account_metas.push(AccountMeta::new(*claim_account.key, false));
        }

        // Add remaining accounts
        account_metas.extend(vec![
            AccountMeta::new(*self.liquidity.key, false),
            AccountMeta::new(*self.liquidity_program.key, false),
            AccountMeta::new_readonly(*self.oracle_program.key, false),
            AccountMeta::new_readonly(*self.supply_token_program.key, false),
            AccountMeta::new_readonly(*self.borrow_token_program.key, false),
            AccountMeta::new_readonly(*self.associated_token_program.key, false),
            AccountMeta::new_readonly(*self.system_program.key, false),
        ]);

        // Add remaining accounts (oracle sources, branches, tick arrays)
        for account in remaining_accounts {
            account_metas.push(AccountMeta::new(*account.key, false));
        }

        let instruction = Instruction {
            program_id: *self.vaults_program.key,
            accounts: account_metas,
            data: instruction_data,
        };

        let mut all_accounts = vec![
            self.signer.clone(),
            self.signer_supply_token_account.clone(),
            self.signer_borrow_token_account.clone(),
            self.recipient.clone(),
            self.recipient_borrow_token_account.clone(),
            self.recipient_supply_token_account.clone(),
            self.vault_config.clone(),
            self.vault_state.clone(),
            self.supply_token.clone(),
            self.borrow_token.clone(),
            self.oracle.clone(),
            self.position.clone(),
            self.position_token_account.clone(),
            self.current_position_tick.clone(),
            self.final_position_tick.clone(),
            self.current_position_tick_id.clone(),
            self.final_position_tick_id.clone(),
            self.new_branch.clone(),
            self.supply_token_reserves_liquidity.clone(),
            self.borrow_token_reserves_liquidity.clone(),
            self.vault_supply_position_on_liquidity.clone(),
            self.vault_borrow_position_on_liquidity.clone(),
            self.supply_rate_model.clone(),
            self.borrow_rate_model.clone(),
            self.vault_supply_token_account.clone(),
            self.vault_borrow_token_account.clone(),
        ];

        // Add optional claim accounts
        if let Some(ref claim_account) = self.supply_token_claim_account {
            all_accounts.push(claim_account.clone());
        }
        if let Some(ref claim_account) = self.borrow_token_claim_account {
            all_accounts.push(claim_account.clone());
        }

        all_accounts.extend(vec![
            self.liquidity.clone(),
            self.liquidity_program.clone(),
            self.oracle_program.clone(),
            self.supply_token_program.clone(),
            self.borrow_token_program.clone(),
            self.associated_token_program.clone(),
            self.system_program.clone(),
        ]);

        // Add remaining accounts
        all_accounts.extend(remaining_accounts);

        invoke(&instruction, &all_accounts)
            .map_err(|_| ErrorCodes::CpiToVaultsProgramFailed.into())?;

        // Return values would need to be parsed from logs or return data
        // For now, returning placeholder values
        Ok((0, new_col, new_debt))
    }
}
```

> Full snippet available [here](../../references/borrow/operate.rs)

---

## Operation Patterns

### 1. Deposit Only

```rust
// Deposit 100 supply tokens
operate_params.operate(
    100_000_000, // new_col (scaled to 1e9)
    0,           // new_debt
    None,        // transfer_type
    vec![oracle_sources_count, branch_count, tick_debt_arrays_count],
    remaining_accounts,
)?;
```

### 2. Deposit + Borrow

```rust
// Deposit 100 supply tokens and borrow 50 borrow tokens
operate_params.operate(
    100_000_000, // new_col (deposit)
    50_000_000,  // new_debt (borrow)
    None,        // transfer_type
    vec![oracle_sources_count, branch_count, tick_debt_arrays_count],
    remaining_accounts,
)?;
```

### 3. Payback + Withdraw

```rust
// Payback 25 borrow tokens and withdraw 50 supply tokens
operate_params.operate(
    -50_000_000, // new_col (withdraw)
    -25_000_000, // new_debt (payback)
    None,        // transfer_type
    vec![oracle_sources_count, branch_count, tick_debt_arrays_count],
    remaining_accounts,
)?;
```

### 4. Max Withdraw

```rust
// Withdraw all available collateral
operate_params.operate(
    i128::MIN, // new_col (max withdraw)
    0,         // new_debt
    None,      // transfer_type
    vec![oracle_sources_count, branch_count, tick_debt_arrays_count],
    remaining_accounts,
)?;
```

### 5. Max Payback

```rust
// Payback all debt
operate_params.operate(
    0,         // new_col
    i128::MIN, // new_debt (max payback)
    None,      // transfer_type
    vec![oracle_sources_count, branch_count, tick_debt_arrays_count],
    remaining_accounts,
)?;
```

---

## Key Implementation Notes

### 1. Amount Scaling

- All amounts are scaled to 1e9 decimals internally
- Use `i128::MIN` for max withdraw/payback operations
- Positive values = deposit/borrow, Negative values = withdraw/payback

### 2. Position Management

- Each user position is represented by an NFT
- Position NFT must be owned by the signer for withdraw/borrow operations
- Anyone can deposit to any position or payback debt for any position

### 3. Remaining Accounts Structure

The `remaining_accounts_indices` vector specifies the count of each account type:

- `indices[0]` = Oracle sources count
- `indices[1]` = Branch accounts count
- `indices[2]` = Tick has debt arrays count

Accounts are ordered in `remaining_accounts` as:

1. Oracle sources (0 to indices[0])
2. Branch accounts (indices[0] to indices[0] + indices[1])
3. Tick has debt arrays (indices[0] + indices[1] to indices[0] + indices[1] + indices[2])

### 4. Transfer Types

- `None` = Normal transfer
- `Some(1)` = Claim type transfer (requires claim accounts)

### 5. Error Handling

Common errors to handle:

- `VaultInvalidOperateAmount`: Operation amount too small or invalid
- `VaultInvalidDecimals`: Token decimals exceed maximum
- `VaultTickIsEmpty`: Position tick has no debt
- `VaultInvalidPaybackOrDeposit`: Invalid payback operation
- `CpiToVaultsProgramFailed`: CPI call failed

### 6. Return Values

The `operate` function returns:

- `nft_id`: Position NFT ID
- `new_col_final`: Final collateral change amount (unscaled)
- `new_debt_final`: Final debt change amount (unscaled)

---

```
---
## `README.md`

```markdown
## Jupiter Lend integration guide

### Earn

- SDK [Guide](./docs/earn/sdk.md)
- CPI [Guide](./docs/earn/cpi.md)
- API [Guide](https://dev.jup.ag/docs/lend-api)

### Borrow

- SDK [Guide](./docs/borrow/sdk.md)
- CPI [Guide](./docs/borrow/cpi.md)

```
