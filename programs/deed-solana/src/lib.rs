use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("7gaQGGmQzUZBsL98WDYLLwWXNkW73DjfMDPYsGFJdbxR");

const MAX_TIERS: usize = 3;

#[program]
pub mod deed_pre_tge {
    use super::*;

    pub fn initialize_sale(
        ctx: Context<InitializeSale>,
        max_supply: u64,
        tiers: Vec<Tier>,
        end_date: i64,
    ) -> Result<()> {
        let sale = &mut ctx.accounts.sale;
        require_gt!(max_supply, 0, DeedError::InvalidMaxSupply);
        require!(!tiers.is_empty() && tiers.len() <= MAX_TIERS, DeedError::InvalidTiers);
        require_gt!(end_date, Clock::get()?.unix_timestamp, DeedError::InvalidEndDate);

        sale.creator = *ctx.accounts.creator.key;
        sale.max_supply = max_supply;
        sale.tiers = tiers;
        sale.end_date = end_date;
        sale.total_sold = 0;
        sale.total_sales_value = 0;
        sale.is_active = true;

        // Transfer SOL to sale_vault and reward_vault to initialize as System Accounts
        let rent = Rent::get()?.minimum_balance(0); // 0-byte account
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                ctx.accounts.creator.key,
                ctx.accounts.sale_vault.key,
                rent,
            ),
            &[
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.sale_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                ctx.accounts.creator.key,
                ctx.accounts.reward_vault.key,
                rent,
            ),
            &[
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.reward_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        emit!(SaleInitialized {
            sale: *ctx.accounts.sale.to_account_info().key,
            sale_vault: *ctx.accounts.sale_vault.to_account_info().key,
            reward_vault: *ctx.accounts.reward_vault.to_account_info().key,
        });

        Ok(())
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, amount: u64, referrer_key: Pubkey) -> Result<()> {
        let sale = &mut ctx.accounts.sale;
        let now = Clock::get()?.unix_timestamp;
        require!(sale.is_active && now < sale.end_date, DeedError::SaleClosed);
        require_gt!(amount, 0, DeedError::InvalidAmount);
        require!(
            sale.total_sold + amount <= sale.max_supply,
            DeedError::MaxSupplyExceeded
        );

        let total_lamports = calculate_total_cost(sale, sale.total_sold, amount)?;
        sale.total_sold = sale.total_sold.checked_add(amount).ok_or(DeedError::MathOverflow)?;
        sale.total_sales_value = sale
            .total_sales_value
            .checked_add(total_lamports)
            .ok_or(DeedError::MathOverflow)?;

        // Transfer SOL to sale_vault (75%) and reward_vault (25%)
        let reward_amount = total_lamports
            .checked_mul(25)
            .ok_or(DeedError::MathOverflow)?
            .checked_div(100)
            .ok_or(DeedError::MathOverflow)?;
        let sale_amount = total_lamports
            .checked_sub(reward_amount)
            .ok_or(DeedError::MathOverflow)?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.sale_vault.to_account_info(),
                },
            ),
            sale_amount,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.reward_vault.to_account_info(),
                },
            ),
            reward_amount,
        )?;

        emit!(SaleEvent {
            buyer: *ctx.accounts.buyer.key,
            referrer: referrer_key,
            amount,
            lamports: total_lamports,
        });

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>, reward_amount: u64) -> Result<()> {
        let sale = &ctx.accounts.sale;
        require!(sale.is_active, DeedError::SaleClosed);
        require_gt!(reward_amount, 0, DeedError::InvalidReward);

        let vault_balance = ctx.accounts.reward_vault.lamports();
        require!(vault_balance >= reward_amount, DeedError::InsufficientFunds);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.reward_vault.to_account_info(),
                    to: ctx.accounts.referrer.to_account_info(),
                },
            ),
            reward_amount,
        )?;

        emit!(RewardClaimed {
            referrer: *ctx.accounts.referrer.key,
            sol_amount: reward_amount,
        });

        Ok(())
    }

    pub fn transfer_excess_rewards(ctx: Context<TransferExcessRewards>, amount: u64) -> Result<()> {
        let sale = &ctx.accounts.sale;
        require!(sale.creator == *ctx.accounts.creator.key, DeedError::Unauthorized);

        require_gt!(amount, 0, DeedError::InvalidAmount);
        let reward_vault_balance = ctx.accounts.reward_vault.lamports();
        require!(reward_vault_balance >= amount, DeedError::InsufficientFunds);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.reward_vault.to_account_info(),
                    to: ctx.accounts.sale_vault.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(ExcessRewardsTransferred { amount });

        Ok(())
    }

    pub fn withdraw_funds(ctx: Context<WithdrawFunds>) -> Result<()> {
        let sale = &mut ctx.accounts.sale;
        let now = Clock::get()?.unix_timestamp;
        require!(!sale.is_active || now >= sale.end_date, DeedError::SaleActive);
        require_eq!(
            sale.creator,
            *ctx.accounts.creator.key,
            DeedError::Unauthorized
        );

        let lamports = ctx.accounts.sale_vault.lamports();
        require_gt!(lamports, 0, DeedError::NoFunds);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.sale_vault.to_account_info(),
                    to: ctx.accounts.creator.to_account_info(),
                },
            ),
            lamports,
        )?;

        sale.is_active = false;

        emit!(FundsWithdrawn {
            creator: *ctx.accounts.creator.key,
            lamports,
        });

        Ok(())
    }

    pub fn close_sale(ctx: Context<CloseSale>) -> Result<()> {
        let sale_account_info = ctx.accounts.sale.to_account_info();
        let creator_account_info = ctx.accounts.creator.to_account_info();

        let expected_pda = Pubkey::find_program_address(
            &[b"sale", creator_account_info.key.as_ref()],
            &ctx.program_id,
        ).0;
        require_keys_eq!(expected_pda, *sale_account_info.key, DeedError::Unauthorized);

        let lamports = sale_account_info.lamports();
        require_gt!(lamports, 0, DeedError::NoFunds);

        **sale_account_info.try_borrow_mut_lamports()? -= lamports;
        **ctx.accounts.creator.try_borrow_mut_lamports()? += lamports;

        emit!(SaleClosed {
            sale: *sale_account_info.key,
            creator: *creator_account_info.key,
            lamports,
        });

        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct TokenSale {
    pub creator: Pubkey,
    pub max_supply: u64,
    pub tiers: Vec<Tier>,
    pub end_date: i64,
    pub total_sold: u64,
    pub total_sales_value: u64,
    pub is_active: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Tier {
    pub amount: u64,
    pub price: u64,
}

#[derive(Accounts)]
pub struct InitializeSale<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 8 + (4 + 3 * 16) + 8 + 8 + 8 + 1,
        seeds = [b"sale", creator.key().as_ref()],
        bump
    )]
    pub sale: Account<'info, TokenSale>,
    #[account(
        mut,
        seeds = [b"sale_vault", sale.key().as_ref()],
        bump
    )]
    pub sale_vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"reward_vault", sale.key().as_ref()],
        bump
    )]
    pub reward_vault: SystemAccount<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: u64, referrer_key: Pubkey)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub sale: Account<'info, TokenSale>,
    #[account(
        mut,
        seeds = [b"sale_vault", sale.key().as_ref()],
        bump
    )]
    pub sale_vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"reward_vault", sale.key().as_ref()],
        bump
    )]
    pub reward_vault: SystemAccount<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub sale: Account<'info, TokenSale>,
    #[account(
        mut,
        seeds = [b"reward_vault", sale.key().as_ref()],
        bump
    )]
    pub reward_vault: SystemAccount<'info>,
    #[account(mut)]
    pub referrer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferExcessRewards<'info> {
    #[account(mut, has_one = creator)]
    pub sale: Account<'info, TokenSale>,
    #[account(
        mut,
        seeds = [b"sale_vault", sale.key().as_ref()],
        bump
    )]
    pub sale_vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"reward_vault", sale.key().as_ref()],
        bump
    )]
    pub reward_vault: SystemAccount<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    #[account(mut)]
    pub sale: Account<'info, TokenSale>,
    #[account(
        mut,
        seeds = [b"sale_vault", sale.key().as_ref()],
        bump
    )]
    pub sale_vault: SystemAccount<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseSale<'info> {
    #[account(mut)]
    pub sale: Account<'info, TokenSale>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct SaleInitialized {
    pub sale: Pubkey,
    pub sale_vault: Pubkey,
    pub reward_vault: Pubkey,
}

#[event]
pub struct SaleEvent {
    pub buyer: Pubkey,
    pub referrer: Pubkey,
    pub amount: u64,
    pub lamports: u64,
}

#[event]
pub struct RewardClaimed {
    pub referrer: Pubkey,
    pub sol_amount: u64,
}

#[event]
pub struct ExcessRewardsTransferred {
    pub amount: u64,
}

#[event]
pub struct FundsWithdrawn {
    pub creator: Pubkey,
    pub lamports: u64,
}

#[event]
pub struct SaleClosed {
    pub sale: Pubkey,
    pub creator: Pubkey,
    pub lamports: u64,
}

#[error_code]
pub enum DeedError {
    #[msg("Sale is closed or expired")]
    SaleClosed,
    #[msg("Max supply exceeded")]
    MaxSupplyExceeded,
    #[msg("Sale is still active")]
    SaleActive,
    #[msg("Invalid reward amount")]
    InvalidReward,
    #[msg("Invalid max supply")]
    InvalidMaxSupply,
    #[msg("Invalid tiers")]
    InvalidTiers,
    #[msg("Invalid end date")]
    InvalidEndDate,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("No funds to withdraw")]
    NoFunds,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
}

fn calculate_total_cost(sale: &TokenSale, total_sold: u64, amount: u64) -> Result<u64> {
    let mut remaining = amount;
    let mut total_cost: u64 = 0;
    let mut sold_so_far = total_sold;

    for tier in &sale.tiers {
        if sold_so_far >= tier.amount {
            sold_so_far -= tier.amount;
            continue;
        }
        let available = tier.amount - sold_so_far;
        let to_buy = remaining.min(available);
        total_cost = total_cost
            .checked_add(to_buy.checked_mul(tier.price).ok_or(DeedError::MathOverflow)?)
            .ok_or(DeedError::MathOverflow)?;
        remaining -= to_buy;
        sold_so_far = 0;
        if remaining == 0 {
            break;
        }
    }
    require_eq!(remaining, 0, DeedError::MaxSupplyExceeded);
    Ok(total_cost)
}