use crate::{id, AccountInfo, ErrorCode, Fee, Pubkey};
use anchor_lang::{
    error,
    error::Error,
    prelude::msg,
    prelude::{Account, Program, Signer, System},
    require,
    solana_program::program::invoke,
    solana_program::{system_instruction, system_program},
    Key, ToAccountInfo,
};
use num_traits::cast::AsPrimitive;
use solana_gateway::{state::GatewayToken, Gateway};
use std::collections::HashMap;

pub const DISCRIMINATOR_SIZE: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U8_SIZE: usize = 1;
pub const U64_SIZE: usize = 8;

pub(crate) const FEE_SEED: &[u8; 23] = br"gateway_derive_fee_seed";
pub(crate) const GATEKEEPER_SEED: &[u8; 22] = br"gateway_derive_gk_seed";

type ParsedGatewayTokenAccountWithFee<'a, 'b> =
    (GatewayToken, u64, Option<Fee>, &'b AccountInfo<'a>);

pub fn matches_gatekeeper_network(
    gateway_token: &GatewayToken,
    gatekeeper_network: &Pubkey,
) -> bool {
    gateway_token.gatekeeper_network == *gatekeeper_network
}

pub fn check_has_matching_gateway_token(
    gateway_tokens: &[ParsedGatewayTokenAccountWithFee],
    gatekeeper_network: &Pubkey,
    expected_owner: &Pubkey,
) -> Result<(), Error> {
    let found_gateway_token = gateway_tokens.iter().find(|(gateway_token, _, _, _)| {
        matches_gatekeeper_network(gateway_token, gatekeeper_network)
    });

    match found_gateway_token {
        Some((gateway_token, balance, _, _)) => Gateway::verify_gateway_token(
            gateway_token,
            expected_owner,
            gatekeeper_network,
            *balance,
            None,
        )
        .map_err(|_| error!(ErrorCode::InvalidComponentPass)),
        _ => Err(error!(ErrorCode::MissingComponentPass)),
    }
}

/// Parse the account token into a fee account.
/// If the fee account is missing, return None
/// If the fee account is not missing, but not owned by the GatewayDerive Program, return an error
fn parse_fee_account<'a, 'b>(
    account_info: &'b AccountInfo<'a>,
    gatekeeper: &Pubkey,
    gatekeeper_network: &Pubkey,
    fee_bump: u8,
) -> Result<Option<Account<'a, Fee>>, Error> {
    let expected_fee_account = derive_fee_address(gatekeeper, gatekeeper_network, fee_bump)
        .map_err(|_| error!(ErrorCode::InvalidFeeAccount))?;

    if expected_fee_account != *account_info.key {
        return Err(error!(ErrorCode::InvalidFeeAccount));
    }

    if account_info.owner == &system_program::id() {
        msg!("Fee account {} not found - not charging", account_info.key);
        if account_info.try_lamports().unwrap() == 0 {
            return Ok(None);
        } else {
            return Err(error!(ErrorCode::InvalidFeeAccount));
        }
    } else if account_info.owner == &id() {
        return Ok(Some(Account::try_from(account_info)?));
    }

    Err(error!(ErrorCode::InvalidFeeAccount))
}

/// Given an array of remaining accounts of the form
/// [gt1, gt2, ... gtN, fee1, fee2, ... feeN, gatekeeper1, gatekeeper2, ... gatekeeperN],
/// return a vector of entries combining the gateway token with the associated fee and gatekeeper
fn parse_accounts<'a, 'b>(
    accounts: &'b [AccountInfo<'a>],
    fee_bumps: &[u8],
) -> Result<Vec<ParsedGatewayTokenAccountWithFee<'a, 'b>>, Error> {
    let gateway_token_count = accounts.len() / 3;

    msg!(
        "Parsing all {} accounts ({} gateway tokens expected)",
        accounts.len(),
        gateway_token_count
    );

    accounts[..gateway_token_count]
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let gateway_token = Gateway::parse_gateway_token(account)
                .map_err(|_| error!(ErrorCode::InvalidComponentPass))?;
            let balance: u64 = account.lamports.borrow().as_();
            let fee: Option<Account<Fee>> = parse_fee_account(
                &accounts[gateway_token_count + i],
                &gateway_token.issuing_gatekeeper,
                &gateway_token.gatekeeper_network,
                fee_bumps[i],
            )?;
            let gatekeeper = &accounts[2 * gateway_token_count + i];

            require!(
                *gatekeeper.key == gateway_token.issuing_gatekeeper,
                ErrorCode::GatekeeperMismatch
            );

            Ok((
                gateway_token,
                balance,
                fee.map(|f| f.into_inner()),
                gatekeeper,
            ))
        })
        .collect::<Result<Vec<ParsedGatewayTokenAccountWithFee<'a, 'b>>, Error>>()
}

/// Given a vector of gateway tokens (GTs) with associated fees and gatekeeper (GK) account objects
/// Returns a map from the gatekeeper key to the lamports (only SOL supported so far) to be sent to it
/// Unlike the input, which has a separate entry per GT, even if several (or all) are issued by the same GK,
/// The output has a single entry per GK, referencing the first AccountInfo that points to it.
pub fn fee_per_gatekeeper<'a, 'b>(
    gateway_tokens_with_fee: Vec<ParsedGatewayTokenAccountWithFee<'a, 'b>>,
) -> HashMap<Pubkey, (&'b AccountInfo<'a>, u64)> {
    let mut fee_map: HashMap<Pubkey, (&'b AccountInfo<'a>, u64)> = HashMap::new();

    for (_, _, fee, gatekeeper) in gateway_tokens_with_fee {
        let current_entry = fee_map.entry(gatekeeper.key()).or_insert((gatekeeper, 0));
        current_entry.1 += fee.map(|f| f.amount).unwrap_or(0);
    }

    fee_map
}

/// Check that each gatekeeper network has a matching gateway token. Errors if either a token is missing or a token is invalid
/// e.g. not parseable, not currently active, not owned by the expected owner, etc.
/// Returns the parsed and validated component passes
pub fn get_validated_component_passes<'a, 'b, 'c>(
    accounts: &'c [AccountInfo<'b>],
    gatekeeper_networks: &'a [Pubkey],
    expected_owner: &'a Pubkey,
    fee_bumps: &[u8],
) -> Result<Vec<ParsedGatewayTokenAccountWithFee<'b, 'c>>, Error> {
    let parsed_gateway_tokens_with_fee = parse_accounts(accounts, fee_bumps)?;

    gatekeeper_networks
        .iter()
        .try_for_each(|gatekeeper_network| {
            check_has_matching_gateway_token(
                &parsed_gateway_tokens_with_fee,
                gatekeeper_network,
                expected_owner,
            )
        })?;

    Ok(parsed_gateway_tokens_with_fee)
}

pub fn validate_empty(
    account: &AccountInfo,
    system_program: &Program<System>,
) -> Result<(), Error> {
    require!(account.data_is_empty(), ErrorCode::NonEmptyAccount);
    require!(
        account.owner == system_program.key,
        ErrorCode::NonEmptyAccount
    );
    Ok(())
}

pub fn derive_fee_address(
    gatekeeper: &Pubkey,
    gatekeeper_network: &Pubkey,
    bump: u8,
) -> Result<Pubkey, Error> {
    Pubkey::create_program_address(
        &[
            FEE_SEED,
            &gatekeeper.to_bytes(),
            &gatekeeper_network.to_bytes(),
            &[bump],
        ],
        &id(),
    )
    .map_err(|_| error!(ErrorCode::InvalidFeeAccount))
}

/// Given a list of gateway tokens with their associated fees and gatekeeper accounts, pay each gatekeeper for their usage
/// bearing in mind that several gateway tokens may have been issued
pub fn pay_gatekeepers<'a, 'b>(
    payer: &mut Signer<'a>,
    parsed_gateway_tokens: Vec<ParsedGatewayTokenAccountWithFee<'a, 'b>>,
    system_program: &AccountInfo<'a>,
) -> Result<u64, Error> {
    let fee_map: HashMap<Pubkey, (&'b AccountInfo<'a>, u64)> =
        fee_per_gatekeeper(parsed_gateway_tokens);
    let mut total_fee = 0;

    fee_map
        .iter()
        .try_for_each::<_, Result<(), Error>>(|(_, (gatekeeper, fee))| {
            let account_infos = &[
                payer.to_account_info(),
                (*gatekeeper).clone().to_account_info(),
                system_program.clone(),
            ];
            msg!(
                "Paying {} lamports from {} to {}",
                fee,
                payer.key,
                gatekeeper.key
            );
            invoke(
                &system_instruction::transfer(payer.key, gatekeeper.key, *fee),
                account_infos,
            )?;

            total_fee += fee;

            Ok(())
        })?;

    Ok(total_fee)
}
