use anchor_lang::{
  error,
  error::Error,
  prelude::msg,
  solana_program::{
    entrypoint::ProgramResult,
    program::invoke_signed,
    sysvar::Sysvar, sysvar
  },
  solana_program::epoch_schedule::Epoch
};
use solana_gateway::{
  instruction::{GatewayInstruction, issue_vanilla, add_gatekeeper},
  state::get_gatekeeper_address_with_seed
};
use crate::{AccountInfo, Pubkey, ErrorCode};

pub(crate) const GATEKEEPER_SEED: &[u8; 22] = br"gateway_derive_gk_seed";

/// Parameters for a CPI Issuing Gateway Tokens
pub struct GatewayTokenIssueParams<'a: 'b, 'b> {
  /// the rent payer
  /// CHECK TODO
  pub payer: AccountInfo<'a>,
  /// the gatekeeper_network that the token is being issued for
  /// CHECK TODO
  pub gatekeeper_network: AccountInfo<'a>,
  /// the recipient of the gateway token
  /// CHECK TODO
  pub recipient: AccountInfo<'a>,
  /// the recipient's gateway token account (to be initialised)
  /// CHECK TODO
  pub gateway_token: AccountInfo<'a>,
  /// the gatekeeper PDA
  /// CHECK TODO
  pub gatekeeper: AccountInfo<'a>,
  /// the gatekeeper account PDA (connecting the gatekeeper to the gk network)
  /// CHECK TODO
  pub gatekeeper_account: AccountInfo<'a>,
  /// the signer seeds for the gatekeeper PDA
  /// CHECK TODO
  pub authority_signer_seeds: &'b [&'b [u8]],
  /// the Gateway program
  /// CHECK TODO
  pub gateway_program: AccountInfo<'a>,
  /// CHECK TODO (can we remove this?)
  pub rent: AccountInfo<'a>,
}

pub fn issue_derived_pass(params: GatewayTokenIssueParams<'_, '_>) -> Result<(), Error> {
  msg!(
        "Issuing a gateway token on network {} to {}",
        params.gatekeeper_network.key,
        params.recipient.key
    );
  invoke_signed(
    &issue_vanilla(
      params.payer.key,
      params.recipient.key,
      params.gatekeeper_account.key,
      params.gatekeeper.key,
      params.gatekeeper_network.key,
      None,
      None, // TODO Tmp
    ),
    &[
      params.payer,
      params.gateway_token,
      params.recipient,
      params.gatekeeper_account,
      params.gatekeeper,
      params.gatekeeper_network,
      params.rent
    ],
    &[params.authority_signer_seeds],
  ).or(Err(error!(ErrorCode::IssueError)))
}

/// Parameters for a CPI Adding a Gatekeeper
pub struct AddGatekeeperParams<'a> {
  /// the rent payer
  /// CHECK TODO
  pub payer: AccountInfo<'a>,
  /// the gatekeeper_network that the gatekeeper is being added to
  /// CHECK TODO
  pub gatekeeper_network: AccountInfo<'a>,
  /// the gatekeeper PDA
  /// CHECK TODO
  pub gatekeeper: AccountInfo<'a>,
  /// the gatekeeper account PDA (connecting the gatekeeper to the gk network)
  /// CHECK TODO
  pub gatekeeper_account: AccountInfo<'a>,
  // /// the signer seeds for the gatekeeper account PDA
  // /// CHECK TODO
  // pub authority_signer_seeds: &'b [&'b [u8]],
  /// the Gateway program
  /// CHECK TODO
  pub gateway_program: AccountInfo<'a>,
  /// CHECK TODO (can we remove this?)
  pub rent: AccountInfo<'a>,
}

pub fn add_derived_gatekeeper(params: AddGatekeeperParams<'_>) -> Result<(), Error> {
  msg!(
        "Adding gatekeeper {} to network {} (registering account {}, payer {})",
    params.gatekeeper.key,
    params.gatekeeper_network.key,
    params.gatekeeper_account.key,
    params.payer.key
  );
  invoke_signed(
    &add_gatekeeper(
      params.payer.key,
      params.gatekeeper.key,
      params.gatekeeper_network.key,
    ),
    &[
      params.payer,
      params.gatekeeper_account,
      params.gatekeeper,
      params.gatekeeper_network,
      params.rent
    ],
    &[],
  ).or(Err(error!(ErrorCode::IssueError)))
}
