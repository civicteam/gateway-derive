mod util;
mod gateway_client;

use anchor_lang::prelude::*;
use solana_gateway::Gateway;
use std::str::FromStr;

declare_id!("dpKGstEdwqh8pDfFh3Qrp1yJ85xbvbZtTcjRaq1yqip");

#[program]
pub mod gateway_derive {
  use crate::gateway_client::{GatewayTokenIssueParams, issue_derived_pass, GATEKEEPER_SEED, add_derived_gatekeeper, AddGatekeeperParams};
  use crate::util::validate_component_passes;
  use super::*;

  pub fn initialize(ctx: Context<Initialize>, source_gkns: Vec<Pubkey>, size: u8, gatekeeper_bump: u8) -> Result<()> {
    msg!("I will derive... {}", size);
    ctx.accounts.derived_pass.version = 0;
    ctx.accounts.derived_pass.authority = *ctx.accounts.authority.key;
    ctx.accounts.derived_pass.gatekeeper_bump = gatekeeper_bump;
    ctx.accounts.derived_pass.source_gkns = source_gkns;

    add_derived_gatekeeper(
      AddGatekeeperParams {
        payer: ctx.accounts.authority.to_account_info(),
        gatekeeper_network: ctx.accounts.derived_pass.to_account_info(),
        gatekeeper: ctx.accounts.derived_gatekeeper.clone(),
        gatekeeper_account: ctx.accounts.derived_gatekeeper_account.clone(),
        // authority_signer_seeds: &[GATEKEEPER_SEED, &ctx.accounts.derived_pass.authority.to_bytes(), &[ctx.accounts.derived_pass.gatekeeper_bump]],
        gateway_program: ctx.accounts.gateway_program.clone(),
        rent: ctx.accounts.rent.to_account_info()
      }
    )?;

    Ok(())
  }

  pub fn issue(ctx: Context<Issue>) -> Result<()> {
    msg!("Issue");
    validate_component_passes(
      ctx.remaining_accounts,
      &ctx.accounts.derived_pass.source_gkns,
      &ctx.accounts.recipient.key
    )?;

    issue_derived_pass(
      GatewayTokenIssueParams {
        payer: ctx.accounts.recipient.to_account_info(),
        gatekeeper_network: ctx.accounts.derived_pass.to_account_info(),
        recipient: ctx.accounts.recipient.to_account_info(),
        gateway_token: ctx.accounts.gateway_token.clone(),
        gatekeeper: ctx.accounts.derived_gatekeeper.clone(),
        gatekeeper_account: ctx.accounts.derived_gatekeeper_account.clone(),
        authority_signer_seeds: &[
          GATEKEEPER_SEED,
          &ctx.accounts.derived_pass.authority.to_bytes(),
          &[ctx.accounts.derived_pass.gatekeeper_bump]
        ],
        gateway_program: ctx.accounts.gateway_program.clone(),
        rent: ctx.accounts.rent.to_account_info()
      }
    )?;

    Ok(())
  }
}

#[account]
pub struct DerivedPass {
  pub version: u8,
  pub authority: Pubkey,
  pub gatekeeper_bump: u8,
  pub source_gkns: Vec<Pubkey>,
}

#[derive(Accounts)]
#[instruction(source_gkns: Vec<Pubkey>, size: u8, gatekeeper_bump: u8)]
pub struct Initialize<'info> {
  #[account(init, payer = authority, space = size.into())]
  derived_pass: Account<'info, DerivedPass>,
  #[account(mut)]
  authority: Signer<'info>, // this is the "gatekeeper network"
  #[account()]
  /// The gatekeeper PDA that this program will use as the signer of gateway transactions.
  /// Derived from the authority address and this program.
  /// Must not exist i.e. owned by the system program and have size = 0
  /// CHECK: TODO above
  derived_gatekeeper: AccountInfo<'info>,
  #[account(mut)]
  /// The gatekeeper account that will be created by this instruction. This indicates to the
  /// gateway program that the derived_gatekeeper is authorised to issue gateway tokens on the
  /// gatekeeper network.
  /// Derived from the gatekeeper address, authority and the gateway program.
  //  Must not exist i.e. owned by the system program and have size = 0
  /// CHECK: TODO above
  derived_gatekeeper_account: AccountInfo<'info>,
  #[account(address = Pubkey::from_str("gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs").unwrap())] // TODO replace with Gateway::id once exposed
  /// CHECK: TODO
  gateway_program: AccountInfo<'info>,
  rent: Sysvar<'info, Rent>,
  system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Issue<'info> {
  #[account()]
  derived_pass: Account<'info, DerivedPass>,
  #[account(mut)]
  recipient: Signer<'info>,
  #[account(mut)]
  /// CHECK: TODO
  gateway_token: AccountInfo<'info>,
  #[account()]
  /// CHECK: TODO
  derived_gatekeeper: AccountInfo<'info>,
  #[account()]
  /// CHECK: TODO
  derived_gatekeeper_account: AccountInfo<'info>,
  #[account(address = Pubkey::from_str("gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs").unwrap())] // TODO replace with Gateway::id once exposed
  /// CHECK: TODO
  gateway_program: AccountInfo<'info>,
  rent: Sysvar<'info, Rent>,
  system_program: Program<'info, System>,
}


#[error_code]
pub enum ErrorCode {
  #[msg("At least one component pass is missing")]
  MissingComponentPass,

  #[msg("At least one of the passed-in component passes is invalid")]
  InvalidComponentPass,

  #[msg("An error occurred during pass issuance")]
  IssueError
}
