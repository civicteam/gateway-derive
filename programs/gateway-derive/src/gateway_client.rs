use crate::{AccountInfo, DerivedPass, ErrorCode, Rent};
use anchor_lang::{
    error,
    error::Error,
    prelude::msg,
    prelude::{Account, Signer, Sysvar},
    solana_program::program::invoke_signed,
    ToAccountInfo,
};
use solana_gateway::instruction::{add_gatekeeper, issue_vanilla};

/// Parameters for a CPI Issuing Gateway Tokens
pub struct GatewayTokenIssueParams<'a: 'b, 'b> {
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
    pub rent: Sysvar<'a, Rent>,
}

pub fn issue_derived_pass(params: GatewayTokenIssueParams<'_, '_>) -> Result<(), Error> {
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
            None, // TODO Tmp
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
