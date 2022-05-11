use anchor_lang::{error, error::Error, prelude::msg, require};
use solana_gateway::{
  Gateway,
  state::GatewayToken,
};
use crate::{AccountInfo, ErrorCode, Pubkey};
use num_traits::cast::AsPrimitive;
use anchor_lang::prelude::{Program, System};

type ParsedGatewayTokenAccount<'a> = (GatewayToken, u64);

pub fn matches_gatekeeper_network(gateway_token: &GatewayToken, gatekeeper_network: &Pubkey) -> bool {
  gateway_token.gatekeeper_network == *gatekeeper_network
}

pub fn check_has_matching_gateway_token<'a>(gateway_tokens: &[ParsedGatewayTokenAccount<'a>], gatekeeper_network: &Pubkey, expected_owner: &Pubkey) -> Result<(), Error> {
  let found_gateway_token = gateway_tokens.iter().find(|(gateway_token, _)| matches_gatekeeper_network(gateway_token, gatekeeper_network));

  msg!("Found Gateway Token for Gatekeeper Network {}: {}", gatekeeper_network, found_gateway_token.is_some());
  match found_gateway_token {
    Some((gateway_token, balance)) => Gateway::verify_gateway_token(
      gateway_token,
      expected_owner,
      gatekeeper_network,
      *balance,
      None
    ).or(Err(error!(ErrorCode::InvalidComponentPass))),
    _ => Err(error!(ErrorCode::MissingComponentPass))
  }
}

/// Check that each gatekeeper network has a matching gateway token. Errors if either a token is missing or a token is invalid
/// e.g. not parseable, not currently active, not owned by the expected owner, etc.
pub fn validate_component_passes(accounts: &[AccountInfo], gatekeeper_networks: &[Pubkey], expected_owner: &Pubkey) -> Result<(), Error> {
  let parsed_gateway_token_accounts: Vec<ParsedGatewayTokenAccount> = accounts
    .iter()
    .map(|account|
      Gateway::parse_gateway_token(account).map(|gateway_token| (
        gateway_token,
        account.lamports.borrow().as_()
      )))
    .collect::<Result<_, _>>()
    .or(Err(error!(ErrorCode::InvalidComponentPass)))?;

  gatekeeper_networks
    .iter()
    .map(|gatekeeper_network| {
      check_has_matching_gateway_token(
        &parsed_gateway_token_accounts,
        gatekeeper_network,
        expected_owner,
      )
    })
    .collect()
}

pub fn validate_empty(account: &AccountInfo, system_program: &Program<System>) -> Result<(), Error> {
  let account_size: u64 = account.lamports.borrow().as_();
  require!(
    account_size == 0,
    ErrorCode::InvalidGatekeeper
  );
  require!(account.owner == system_program.key, ErrorCode::InvalidGatekeeper);
  Ok(())
}
