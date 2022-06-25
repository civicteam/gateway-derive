use crate::{AccountInfo, DerivedPass, ErrorCode, Rent, UnixTimestamp};
use anchor_lang::prelude::Program;
use anchor_lang::system_program::System;
use anchor_lang::{
    error,
    error::Error,
    prelude::msg,
    prelude::{Account, Signer, Sysvar},
    solana_program::program::invoke_signed,
    Key, ToAccountInfo,
};
use solana_gateway::instruction::{
    add_feature_to_network, add_gatekeeper, issue_vanilla, update_expiry, NetworkFeature,
};
use solana_gateway::state::get_expire_address_with_seed;

/// Parameters for a CPI operation on Gateway Tokens
pub struct GatewayTokenParams<'a: 'b, 'b> {
    /// the rent payer
    pub payer: Signer<'a>,
    /// the gatekeeper_network that the token is being issued for
    pub gatekeeper_network: Account<'a, DerivedPass>,
    /// the recipient of the gateway token
    pub recipient: Signer<'a>,
    /// the recipient's gateway token account (to be initialised)
    /// CHECK Verified by the Gateway program during the CPI call
    pub gateway_token: AccountInfo<'a>,
    /// the gatekeeper PDA
    /// CHECK Verified by the Gateway program during the CPI call
    pub gatekeeper: AccountInfo<'a>,
    /// the gatekeeper account PDA (connecting the gatekeeper to the gk network)
    /// CHECK Verified by the Gateway program during the CPI call
    pub gatekeeper_account: AccountInfo<'a>,
    /// the signer seeds for the gatekeeper PDA
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// An optional expiration timestamp to add to the token
    pub expire_time: Option<UnixTimestamp>,
    pub rent: Sysvar<'a, Rent>,
}

pub fn issue_derived_pass(params: GatewayTokenParams<'_, '_>) -> Result<(), Error> {
    msg!(
        "Issuing a gateway token on network {} to {}",
        params.gatekeeper_network.to_account_info().key,
        params.recipient.key
    );
    invoke_signed(
        &issue_vanilla(
            params.payer.key,
            params.recipient.key,
            params.gatekeeper_account.key,
            params.gatekeeper.key,
            params.gatekeeper_network.to_account_info().key,
            None,
            params.expire_time,
        ),
        &[
            params.payer.to_account_info(),
            params.gateway_token,
            params.recipient.to_account_info(),
            params.gatekeeper_account,
            params.gatekeeper,
            params.gatekeeper_network.to_account_info(),
            params.rent.to_account_info(),
        ],
        &[params.authority_signer_seeds],
    )
    .map_err(|_| error!(ErrorCode::IssueError))
}

pub fn refresh_derived_pass(params: GatewayTokenParams<'_, '_>) -> Result<(), Error> {
    msg!(
        "Refreshing gateway token {} on network {} for {}",
        params.gateway_token.key,
        params.gatekeeper_network.to_account_info().key,
        params.recipient.key
    );

    let expire_time = params
        .expire_time
        .ok_or_else(|| error!(ErrorCode::MissingExpireTime))?;

    invoke_signed(
        &update_expiry(
            params.gateway_token.key,
            params.gatekeeper.key,
            params.gatekeeper_account.key,
            expire_time,
        ),
        &[
            params.gateway_token,
            params.gatekeeper,
            params.gatekeeper_account,
        ],
        &[params.authority_signer_seeds],
    )
    .map_err(|_| error!(ErrorCode::RefreshError))
}

/// Parameters for a CPI Adding a Gatekeeper
pub struct AddGatekeeperParams<'a> {
    /// the rent payer
    pub payer: Signer<'a>,
    /// the gatekeeper_network that the token is being issued for
    pub gatekeeper_network: Account<'a, DerivedPass>,
    /// the gatekeeper PDA
    /// CHECK Already verified by the program at this point
    pub gatekeeper: AccountInfo<'a>,
    /// the gatekeeper account PDA (connecting the gatekeeper to the gk network)
    /// CHECK Already verified by the program at this point
    pub gatekeeper_account: AccountInfo<'a>,
    pub rent: Sysvar<'a, Rent>,
}

pub fn add_derived_gatekeeper(params: AddGatekeeperParams<'_>) -> Result<(), Error> {
    let gatekeeper_network = params.gatekeeper_network.to_account_info();
    invoke_signed(
        &add_gatekeeper(
            params.payer.key,
            params.gatekeeper.key,
            gatekeeper_network.key,
        ),
        &[
            params.payer.to_account_info(),
            params.gatekeeper_account,
            params.gatekeeper,
            gatekeeper_network,
            params.rent.to_account_info(),
        ],
        &[],
    )
    .map_err(|_| error!(ErrorCode::IssueError))
}

/// Parameters for a CPI Adding the ExpirableOnUse feature to a gatekeeper network
pub struct AddExpirableOnUseParams<'a> {
    /// the rent payer
    pub payer: Signer<'a>,
    /// the gatekeeper_network that the token is being issued for
    pub gatekeeper_network: Account<'a, DerivedPass>,
    /// the account whose presence indicates that a token is expirable.
    /// CHECK: Derivation is checked inside add_expirable_on_use
    pub feature_account: AccountInfo<'a>,
    pub system_program: Program<'a, System>,
}

pub fn add_expirable_on_use(params: AddExpirableOnUseParams<'_>) -> Result<(), Error> {
    let payer = params.payer.to_account_info();
    let gatekeeper_network = params.gatekeeper_network.to_account_info();
    let system_program = params.system_program.to_account_info();

    let feature_account_key = get_expire_address_with_seed(&gatekeeper_network.key()).0;
    if !feature_account_key.eq(params.feature_account.key) {
        return Err(error!(ErrorCode::InvalidFeatureAccount));
    }

    invoke_signed(
        &add_feature_to_network(
            *payer.key,
            gatekeeper_network.key(),
            NetworkFeature::UserTokenExpiry,
        ),
        &[
            payer,
            gatekeeper_network,
            params.feature_account,
            system_program,
        ],
        &[],
    )
    .map_err(|_| error!(ErrorCode::IssueError))
}
