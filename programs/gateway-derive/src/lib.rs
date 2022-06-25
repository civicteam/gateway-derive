mod gateway_client;
mod util;

use crate::{
    gateway_client::{
        add_derived_gatekeeper, issue_derived_pass, AddGatekeeperParams, GatewayTokenParams,
    },
    util::{DISCRIMINATOR_SIZE, FEE_SEED, GATEKEEPER_SEED, PUBKEY_SIZE, U64_SIZE, U8_SIZE},
};
use anchor_lang::{prelude::*, solana_program::clock::UnixTimestamp};
use std::borrow::BorrowMut;
use std::str::FromStr;

#[macro_use]
extern crate num_derive;

declare_id!("dpKGstEdwqh8pDfFh3Qrp1yJ85xbvbZtTcjRaq1yqip");

#[derive(Debug, Clone)]
pub struct Gateway;

impl Id for Gateway {
    fn id() -> Pubkey {
        Pubkey::from_str("gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs").unwrap()
    }
}

#[program]
pub mod gateway_derive {
    use super::*;
    use crate::gateway_client::refresh_derived_pass;
    use crate::util::{create_or_update_fee, validate_gateway_token, Action};
    use crate::{
        gateway_client::{add_expirable_on_use, AddExpirableOnUseParams},
        util::{
            get_expiry_time, get_validated_component_passes, pay_gatekeepers, validate_empty,
            GATEKEEPER_SEED,
        },
    };

    pub fn initialize<'info>(
        ctx: Context<'_, '_, '_, 'info, Initialize<'info>>,
        source_gkns: Vec<Pubkey>,
        _size: u8,
        gatekeeper_bump: u8,
        properties: DerivedPassProperties,
    ) -> Result<()> {
        ctx.accounts.derived_pass.version = 0;
        ctx.accounts.derived_pass.authority = *ctx.accounts.authority.key;
        ctx.accounts.derived_pass.gatekeeper_bump = gatekeeper_bump;
        ctx.accounts.derived_pass.source_gkns = source_gkns;
        ctx.accounts.derived_pass.properties = properties;

        let mut remaining_accounts = ctx.remaining_accounts.iter();

        // ensure the gatekeeper account is empty
        let derived_gatekeeper = ctx.accounts.derived_gatekeeper.to_account_info();
        let derived_gatekeeper_account = ctx.accounts.derived_gatekeeper_account.to_account_info();
        let system_program = &ctx.accounts.system_program;
        validate_empty(&derived_gatekeeper, system_program)?;
        validate_empty(&derived_gatekeeper_account, system_program)?;

        add_derived_gatekeeper(AddGatekeeperParams {
            payer: ctx.accounts.authority.clone(),
            gatekeeper_network: ctx.accounts.derived_pass.clone(),
            gatekeeper: derived_gatekeeper,
            gatekeeper_account: derived_gatekeeper_account,
            rent: ctx.accounts.rent.clone(),
        })?;

        if properties.expire_on_use {
            let feature_account = remaining_accounts.next().unwrap();
            add_expirable_on_use(AddExpirableOnUseParams {
                payer: ctx.accounts.authority.clone(),
                gatekeeper_network: ctx.accounts.derived_pass.clone(),
                feature_account: feature_account.clone(),
                system_program: ctx.accounts.system_program.clone(),
            })?;
        }

        Ok(())
    }

    pub fn issue<'info>(
        ctx: Context<'_, '_, '_, 'info, Issue<'info>>,
        fee_bumps: Vec<u8>,
    ) -> Result<()> {
        let system_program = &ctx.accounts.system_program;
        let gateway_token = ctx.accounts.gateway_token.to_account_info();
        validate_empty(&gateway_token, system_program)?;

        if fee_bumps.len() != ctx.remaining_accounts.len() / 3 {
            return Err(error!(ErrorCode::IncorrectFeeBumpCount));
        }

        let parsed_component_passes = get_validated_component_passes(
            ctx.remaining_accounts,
            &ctx.accounts.derived_pass.source_gkns,
            ctx.accounts.recipient.key,
            fee_bumps.as_slice(),
        )?;

        pay_gatekeepers(
            &mut ctx.accounts.recipient,
            parsed_component_passes,
            &ctx.accounts.system_program.to_account_info(),
            Action::Issue,
        )?;

        issue_derived_pass(GatewayTokenParams {
            payer: ctx.accounts.recipient.clone(),
            gatekeeper_network: ctx.accounts.derived_pass.clone(),
            recipient: ctx.accounts.recipient.clone(),
            gateway_token,
            gatekeeper: ctx.accounts.derived_gatekeeper.to_account_info(),
            gatekeeper_account: ctx.accounts.derived_gatekeeper_account.to_account_info(),
            authority_signer_seeds: &[
                GATEKEEPER_SEED,
                &ctx.accounts.derived_pass.authority.to_bytes(),
                &[ctx.accounts.derived_pass.gatekeeper_bump],
            ],
            expire_time: get_expiry_time(ctx.accounts.derived_pass.properties.expire_duration),
            rent: ctx.accounts.rent.clone(),
        })?;

        Ok(())
    }

    pub fn refresh<'info>(
        ctx: Context<'_, '_, '_, 'info, Refresh<'info>>,
        fee_bumps: Vec<u8>,
    ) -> Result<()> {
        require!(
            !ctx.accounts.derived_pass.properties.refresh_disabled,
            ErrorCode::RefreshDisabled
        );
        let gateway_program = &ctx.accounts.gateway_program;
        let gateway_token = ctx.accounts.gateway_token.to_account_info();
        validate_gateway_token(&gateway_token, gateway_program)?;

        if fee_bumps.len() != ctx.remaining_accounts.len() / 3 {
            return Err(error!(ErrorCode::IncorrectFeeBumpCount));
        }

        let parsed_component_passes = get_validated_component_passes(
            ctx.remaining_accounts,
            &ctx.accounts.derived_pass.source_gkns,
            ctx.accounts.recipient.key,
            fee_bumps.as_slice(),
        )?;

        pay_gatekeepers(
            &mut ctx.accounts.recipient,
            parsed_component_passes,
            &ctx.accounts.system_program.to_account_info(),
            Action::Refresh,
        )?;

        refresh_derived_pass(GatewayTokenParams {
            payer: ctx.accounts.recipient.clone(),
            gatekeeper_network: ctx.accounts.derived_pass.clone(),
            recipient: ctx.accounts.recipient.clone(),
            gateway_token,
            gatekeeper: ctx.accounts.derived_gatekeeper.to_account_info(),
            gatekeeper_account: ctx.accounts.derived_gatekeeper_account.to_account_info(),
            authority_signer_seeds: &[
                GATEKEEPER_SEED,
                &ctx.accounts.derived_pass.authority.to_bytes(),
                &[ctx.accounts.derived_pass.gatekeeper_bump],
            ],
            expire_time: get_expiry_time(ctx.accounts.derived_pass.properties.expire_duration),
            rent: ctx.accounts.rent.clone(),
        })?;

        Ok(())
    }

    pub fn create_fee(
        ctx: Context<CreateFee>,
        issue_amount: u64,
        refresh_amount: u64,
        percentage: u8,
        fee_type: u8, // Type: FeeType- Anchor does not yet provide mappings for enums
        mint: Option<Pubkey>,
    ) -> Result<()> {
        create_or_update_fee(
            ctx.accounts.fee.borrow_mut(),
            issue_amount,
            refresh_amount,
            percentage,
            fee_type,
            mint,
        );
        Ok(())
    }

    pub fn update_fee(
        ctx: Context<UpdateFee>,
        issue_amount: u64,
        refresh_amount: u64,
        percentage: u8,
        fee_type: u8, // Type: FeeType- Anchor does not yet provide mappings for enums
        mint: Option<Pubkey>,
    ) -> Result<()> {
        create_or_update_fee(
            ctx.accounts.fee.borrow_mut(),
            issue_amount,
            refresh_amount,
            percentage,
            fee_type,
            mint,
        );
        Ok(())
    }

    pub fn remove_fee(_ctx: Context<RemoveFee>) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct DerivedPassProperties {
    /// The amount of time in seconds that the derived pass is valid for.
    pub expire_duration: Option<i64>, // i64 because that is the type of clock.unix_timestamp
    /// If true, the derived pass can be immediately expired after use
    pub expire_on_use: bool,
    /// If false, the derived pass cannot be refreshed.
    /// Use this for "single-use" passes.
    pub refresh_disabled: bool,
}

#[account]
pub struct DerivedPass {
    pub version: u8,
    pub authority: Pubkey,
    pub gatekeeper_bump: u8,
    pub source_gkns: Vec<Pubkey>,
    pub properties: DerivedPassProperties,
}

#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize, FromPrimitive)]
pub enum FeeType {
    IssuerOnly = 0,
    // TODO support revenue share to derived pass authority
}
impl Default for FeeType {
    fn default() -> Self {
        FeeType::IssuerOnly
    }
}

#[account]
pub struct Fee {
    pub version: u8,
    pub fee_type: FeeType,
    pub percentage: u8, // ignored if type = IssuerOnly - added now for future-compatibility
    pub issue_amount: u64,
    pub refresh_amount: u64,
    pub mint: Option<Pubkey>,
}
impl Fee {
    pub fn get_space() -> usize {
        DISCRIMINATOR_SIZE + (3 * U8_SIZE) + U64_SIZE + U64_SIZE + PUBKEY_SIZE + 1
        // mint: Optional marker adds 1 byte
    }
}

#[derive(Accounts)]
#[instruction(source_gkns: Vec<Pubkey>, size: u8, gatekeeper_bump: u8, properties: DerivedPassProperties)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = size.into())]
    derived_pass: Account<'info, DerivedPass>,
    #[account(mut)]
    authority: Signer<'info>, // this is the "gatekeeper network"
    #[account(seeds = [GATEKEEPER_SEED, &authority.key.to_bytes()], bump = gatekeeper_bump)]
    /// The gatekeeper PDA that this program will use as the signer of gateway transactions.
    /// Derived from the authority address and this program.
    /// Must not exist i.e. owned by the system program and have size = 0
    /// CHECK: The seed derivation is checked here - the size = 0 is checked in the program.
    derived_gatekeeper: UncheckedAccount<'info>,
    #[account(mut)]
    // #[account(mut, seeds = [&authority.key.to_bytes(), &derived_pass.key.to_bytes()], bump = gatekeeper_account_bump )]
    // we do not know what the gatekeeper_account_bump is, the gateway lib does not expose it, so do not check this here
    // we are happy as long as the account is owned by the system program and is empty
    /// The gatekeeper account that will be created by this instruction. This indicates to the
    /// gateway program that the derived_gatekeeper is authorised to issue gateway tokens on the
    /// gatekeeper network.
    /// Derived from the gatekeeper address, authority and the gateway program.
    //  Must not exist i.e. owned by the system program and have size = 0
    /// CHECK: Size and owner is checked in the program - the derivation is checked in the gateway program.
    derived_gatekeeper_account: UncheckedAccount<'info>,
    gateway_program: Program<'info, Gateway>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee_bumps: Vec<u8>)]
pub struct Issue<'info> {
    #[account()]
    derived_pass: Account<'info, DerivedPass>,
    #[account(mut)]
    recipient: Signer<'info>,
    #[account(mut)]
    /// The gateway token that will be created and issued to the recipient.
    /// Derived from the recipient address, the gateway program and an optional seed (empty here).
    ///  Must not exist i.e. owned by the system program and have size = 0
    /// CHECK: Size and owner is checked in the program - the derivation is checked in the gateway program.
    gateway_token: UncheckedAccount<'info>,
    #[account()]
    /// A PDA representing the gatekeeper.
    /// CHECK: Checked in the CPI to the Gateway program
    derived_gatekeeper: UncheckedAccount<'info>,
    #[account(owner = Gateway::id())]
    /// The account linking the derived gatekeeper to the derived_pass gatekeeper network
    /// CHECK: Checked in the CPI to the Gateway program
    derived_gatekeeper_account: UncheckedAccount<'info>,
    gateway_program: Program<'info, Gateway>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee_bumps: Vec<u8>)]
pub struct Refresh<'info> {
    #[account()]
    derived_pass: Account<'info, DerivedPass>,
    #[account(mut)]
    recipient: Signer<'info>,
    #[account(mut)]
    /// The gateway token to be refreshed
    ///  Must be a valid gateway token owned by the recipient
    /// CHECK: the derivation is checked in the gateway program.
    gateway_token: UncheckedAccount<'info>,
    #[account()]
    /// A PDA representing the gatekeeper.
    /// CHECK: Checked in the CPI to the Gateway program
    derived_gatekeeper: UncheckedAccount<'info>,
    #[account(owner = Gateway::id())]
    /// The account linking the derived gatekeeper to the derived_pass gatekeeper network
    /// CHECK: Checked in the CPI to the Gateway program
    derived_gatekeeper_account: UncheckedAccount<'info>,
    gateway_program: Program<'info, Gateway>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(issue_amount: u64, refresh_amount: u64, percentage: u8, fee_type: u8, mint: Option<Pubkey>)]
pub struct CreateFee<'info> {
    #[account(
  init,
  payer = authority,
  space = Fee::get_space(),
  seeds = [FEE_SEED.as_ref(), authority.key.to_bytes().as_ref(), gatekeeper_network.key.to_bytes().as_ref()],
  bump
  )]
    fee: Account<'info, Fee>,
    #[account(mut)]
    authority: Signer<'info>, // the gatekeeper
    /// CHECK: This can be any public key (in reality it should match a known gatekeeper network)
    gatekeeper_network: UncheckedAccount<'info>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(issue_amount: u64, refresh_amount: u64, percentage: u8, fee_type: u8, mint: Option<Pubkey>)]
pub struct UpdateFee<'info> {
    #[account(mut, seeds = [FEE_SEED, &authority.key.to_bytes(), &gatekeeper_network.key.to_bytes()], bump)]
    fee: Account<'info, Fee>,
    #[account(mut)]
    authority: Signer<'info>, // the gatekeeper
    /// CHECK: This can be any public key (in reality it should match a known gatekeeper network)
    gatekeeper_network: UncheckedAccount<'info>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveFee<'info> {
    #[account(mut, close = authority, seeds = [FEE_SEED, &authority.key.to_bytes(), &gatekeeper_network.key.to_bytes()], bump)]
    fee: Account<'info, Fee>,
    #[account(mut)]
    authority: Signer<'info>, // the gatekeeper
    /// CHECK: This can be any public key (in reality it should match a known gatekeeper network)
    gatekeeper_network: UncheckedAccount<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("At least one component pass is missing")]
    MissingComponentPass,

    #[msg("At least one of the passed-in component passes is invalid")]
    InvalidComponentPass,

    #[msg("An error occurred during pass issuance")]
    IssueError,

    #[msg("An error occurred during pass refresh")]
    RefreshError,

    #[msg("Attempt to refresh a pass whose refresh is disabled")]
    RefreshDisabled,

    #[msg("The passed account must be empty")]
    NonEmptyAccount,

    #[msg("A gatekeeper account was passed that does not match the associated component pass gatekeeper")]
    GatekeeperMismatch,

    #[msg("At least one of the passed-in fee accounts is invalid")]
    InvalidFeeAccount,

    #[msg("An overflow error occurred during payment")]
    PaymentOverflow,

    #[msg("An underflow error occurred during payment")]
    PaymentUnderflow,

    #[msg("The list of fee bumps must be equal to the number of component gateway tokens")]
    IncorrectFeeBumpCount,

    #[msg("The feature account does not match the gateway feature")]
    InvalidFeatureAccount,

    #[msg("Missing expire time on refresh")]
    MissingExpireTime,

    #[msg("Invalid gateway token")]
    InvalidGatewayToken,
}
