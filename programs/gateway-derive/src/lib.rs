mod gateway_client;
mod util;

use crate::gateway_client::{
    add_derived_gatekeeper, issue_derived_pass, AddGatekeeperParams, GatewayTokenIssueParams,
};
use crate::util::{DISCRIMINATOR_SIZE, FEE_SEED, GATEKEEPER_SEED, PUBKEY_SIZE, U64_SIZE, U8_SIZE};
use anchor_lang::prelude::*;
use std::str::FromStr;

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
    use crate::util::{
        get_validated_component_passes, pay_gatekeepers, validate_empty, GATEKEEPER_SEED,
    };

    pub fn initialize(
        ctx: Context<Initialize>,
        source_gkns: Vec<Pubkey>,
        _size: u8,
        gatekeeper_bump: u8,
    ) -> Result<()> {
        ctx.accounts.derived_pass.version = 0;
        ctx.accounts.derived_pass.authority = *ctx.accounts.authority.key;
        ctx.accounts.derived_pass.gatekeeper_bump = gatekeeper_bump;
        ctx.accounts.derived_pass.source_gkns = source_gkns;

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
        )?;

        issue_derived_pass(GatewayTokenIssueParams {
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
            rent: ctx.accounts.rent.clone(),
        })?;

        Ok(())
    }

    pub fn create_fee(ctx: Context<CreateFee>, amount: u64, mint: Option<Pubkey>) -> Result<()> {
        ctx.accounts.fee.version = 0;
        ctx.accounts.fee.amount = amount;
        ctx.accounts.fee.mint = mint;
        Ok(())
    }

    pub fn update_fee(ctx: Context<UpdateFee>, amount: u64, mint: Option<Pubkey>) -> Result<()> {
        ctx.accounts.fee.amount = amount;
        ctx.accounts.fee.mint = mint;
        Ok(())
    }

    pub fn remove_fee(_ctx: Context<RemoveFee>) -> Result<()> {
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

#[account]
pub struct Fee {
    pub version: u8,
    pub amount: u64,
    pub mint: Option<Pubkey>,
}

impl Fee {
    pub fn get_space() -> usize {
        DISCRIMINATOR_SIZE + U8_SIZE + U64_SIZE + PUBKEY_SIZE + 1 // mint: Optional marker adds 1 byte
    }
}

#[derive(Accounts)]
#[instruction(source_gkns: Vec<Pubkey>, size: u8, gatekeeper_bump: u8)]
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
#[instruction(amount: u64, mint: Option<Pubkey>)]
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
#[instruction(amount: u64, mint: Option<Pubkey>)]
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
}
