use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("CJs8wLQAVkur36gabN4BsX6Wfjnz3n7fEtia51vpoQDt");

#[program]
pub mod automated_market_maker {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Result<()> {
        ctx.accounts
            .setup_pool(fee_numerator, fee_denominator, &ctx.bumps)
    }

    pub fn add_liquidity(ctx: Context<AddLiquidity>, amount_a: u64, amount_b: u64) -> Result<()> {
        ctx.accounts.execute_add_liquidity(amount_a, amount_b)
    }

    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, lp_tokens: u64) -> Result<()> {
        ctx.accounts.execute_remove_liquidity(lp_tokens)
    }

    pub fn swap(ctx: Context<Swap>, input_amount: u64, minimum_output: u64, is_a_to_b: bool) -> Result<()> {
        ctx.accounts.execute_swap(input_amount, minimum_output, is_a_to_b)
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        space = 8 + Pool::INIT_SPACE,
        bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = lp_mint.decimals == token_a_mint.decimals && 
                    lp_mint.decimals == token_b_mint.decimals
    )]
    pub lp_mint: Account<'info, Mint>,

    pub token_a_mint: Account<'info, Mint>,
    pub token_b_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

impl<'info> InitializePool<'info> {
    pub fn setup_pool(
        &mut self,
        fee_numerator: u64,
        fee_denominator: u64,
        bumps: &InitializePoolBumps,
    ) -> Result<()> {
        if fee_denominator == 0 {
            return Err(AmmError::InvalidFee.into());
        }

        let pool = &mut self.pool;
        pool.token_a_reserve = 0;
        pool.token_b_reserve = 0;
        pool.lp_mint = self.lp_mint.key();
        pool.fee_numerator = fee_numerator;
        pool.fee_denominator = fee_denominator;
        pool.bump = bumps.pool;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(
        mut,
        seeds = [b"pool", token_a_reserve.mint.key().as_ref(), token_b_reserve.mint.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = user_lp_token.mint == lp_mint.key(),
        constraint = user_lp_token.owner == authority.key()
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_a_reserve.mint == pool.token_a_mint
    )]
    pub token_a_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = token_b_reserve.mint == pool.token_b_mint
    )]
    pub token_b_reserve: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.mint == pool.token_a_mint,
        constraint = user_token_a.owner == authority.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_b.mint == pool.token_b_mint,
        constraint = user_token_b.owner == authority.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> AddLiquidity<'info> {
    pub fn execute_add_liquidity(&mut self, amount_a: u64, amount_b: u64) -> Result<()> {
        if amount_a == 0 || amount_b == 0 {
            return Err(AmmError::InvalidAmount.into());
        }

        if self.pool.token_a_reserve.checked_add(amount_a).is_none()
            || self.pool.token_b_reserve.checked_add(amount_b).is_none()
        {
            return Err(AmmError::ArithmeticError.into());
        }

        let transfer_a_ctx = self.transfer_a_ctx();
        let transfer_b_ctx = self.transfer_b_ctx();
        let token_a_reserve = self.token_a_reserve.mint.key();
        let token_b_reserve = self.token_b_reserve.mint.key();
        let signer_seeds = &[
            b"pool".as_ref(),
            token_a_reserve.as_ref(),
            token_b_reserve.as_ref(),
            &[self.pool.bump],
        ];

        // Transfer tokens
        token::transfer(transfer_a_ctx, amount_a)?;
        token::transfer(transfer_b_ctx, amount_b)?;

        let pool = &mut self.pool;

        // Mint LP tokens
        let lp_tokens_to_mint = if pool.token_a_reserve == 0 || pool.token_b_reserve == 0 {
            amount_a + amount_b
        } else {
            let a_share = amount_a * pool.total_lp_tokens / pool.token_a_reserve;
            let b_share = amount_b * pool.total_lp_tokens / pool.token_b_reserve;
            a_share.min(b_share)
        };

        if lp_tokens_to_mint == 0 {
            return Err(AmmError::ArithmeticError.into());
        }

        pool.token_a_reserve += amount_a;
        pool.token_b_reserve += amount_b;
        pool.total_lp_tokens += lp_tokens_to_mint;

        token::mint_to(
            self.mint_to_ctx().with_signer(&[signer_seeds]),
            lp_tokens_to_mint,
        )?;
        Ok(())
    }

    pub fn transfer_a_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_token_a.to_account_info(),
            to: self.token_a_reserve.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn transfer_b_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_token_b.to_account_info(),
            to: self.token_b_reserve.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn mint_to_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.lp_mint.to_account_info(),
            to: self.user_lp_token.to_account_info(),
            authority: self.pool.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(
        mut,
        seeds = [b"pool", token_a_reserve.mint.key().as_ref(), token_b_reserve.mint.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = user_lp_token.mint == lp_mint.key(),
        constraint = user_lp_token.owner == authority.key()
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_a_reserve.mint == pool.token_a_mint
    )]
    pub token_a_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = token_b_reserve.mint == pool.token_b_mint
    )]
    pub token_b_reserve: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.mint == pool.token_a_mint,
        constraint = user_token_a.owner == authority.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_b.mint == pool.token_b_mint,
        constraint = user_token_b.owner == authority.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> RemoveLiquidity<'info> {
    pub fn execute_remove_liquidity(&mut self, lp_tokens: u64) -> Result<()> {
        if lp_tokens == 0 {
            return Err(AmmError::InvalidAmount.into());
        }

        let pool = &mut self.pool;

        // Calculate shares
        let amount_a = lp_tokens * pool.token_a_reserve / pool.total_lp_tokens;
        let amount_b = lp_tokens * pool.token_b_reserve / pool.total_lp_tokens;

        pool.token_a_reserve -= amount_a;
        pool.token_b_reserve -= amount_b;
        pool.total_lp_tokens -= lp_tokens;

        // Burn LP tokens
        token::burn(self.burn_ctx(), lp_tokens)?;

        // Transfer tokens back to the user
        token::transfer(self.transfer_a_ctx(), amount_a)?;
        token::transfer(self.transfer_b_ctx(), amount_b)?;
        Ok(())
    }

    pub fn burn_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.lp_mint.to_account_info(),
            from: self.user_lp_token.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn transfer_a_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_token_a.to_account_info(),
            to: self.token_a_reserve.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn transfer_b_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_token_b.to_account_info(),
            to: self.token_b_reserve.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(
        mut,
        seeds = [b"pool", token_a_reserve.mint.key().as_ref(), token_b_reserve.mint.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = token_a_reserve.mint == pool.token_a_mint
    )]
    pub token_a_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = token_b_reserve.mint == pool.token_b_mint
    )]
    pub token_b_reserve: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.mint == pool.token_a_mint,
        constraint = user_token_a.owner == authority.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_b.mint == pool.token_b_mint,
        constraint = user_token_b.owner == authority.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    pub fn execute_swap(&mut self, input_amount: u64, minimum_output: u64, is_a_to_b: bool) -> Result<()> {
        if input_amount == 0 {
            return Err(AmmError::InvalidAmount.into());
        }

        let pool = &mut self.pool;
        
        // Calculate fee and net input
        let fee = input_amount
            .checked_mul(pool.fee_numerator)
            .ok_or(AmmError::ArithmeticError)?
            .checked_div(pool.fee_denominator)
            .ok_or(AmmError::ArithmeticError)?;
            
        let net_input = input_amount
            .checked_sub(fee)
            .ok_or(AmmError::ArithmeticError)?;

        // Calculate output amount using constant product formula
        let output_amount = self.calculate_output_amount(net_input, is_a_to_b)?;
        
        // Check minimum output
        if output_amount < minimum_output {
            return Err(AmmError::SlippageExceeded.into());
        }

        // Update reserves
        self.update_reserves(input_amount, output_amount, is_a_to_b)?;

        // Execute transfers
        self.execute_transfers(input_amount, output_amount, is_a_to_b)?;

        Ok(())
    }

    fn calculate_output_amount(&self, net_input: u64, is_a_to_b: bool) -> Result<u64> {
        let pool = &self.pool;
        let (input_reserve, output_reserve) = if is_a_to_b {
            (pool.token_a_reserve, pool.token_b_reserve)
        } else {
            (pool.token_b_reserve, pool.token_a_reserve)
        };

        // Using constant product formula: x * y = k
        let output_amount = output_reserve
            .checked_mul(net_input)
            .ok_or(AmmError::ArithmeticError)?
            .checked_div(input_reserve.checked_add(net_input).ok_or(AmmError::ArithmeticError)?)
            .ok_or(AmmError::ArithmeticError)?;

        if output_amount == 0 {
            return Err(AmmError::ZeroSwapOutput.into());
        }

        Ok(output_amount)
    }

    fn update_reserves(&mut self, input: u64, output: u64, is_a_to_b: bool) -> Result<()> {
        let pool = &mut self.pool;
        if is_a_to_b {
            pool.token_a_reserve = pool.token_a_reserve
                .checked_add(input)
                .ok_or(AmmError::ArithmeticError)?;
            pool.token_b_reserve = pool.token_b_reserve
                .checked_sub(output)
                .ok_or(AmmError::ArithmeticError)?;
        } else {
            pool.token_b_reserve = pool.token_b_reserve
                .checked_add(input)
                .ok_or(AmmError::ArithmeticError)?;
            pool.token_a_reserve = pool.token_a_reserve
                .checked_sub(output)
                .ok_or(AmmError::ArithmeticError)?;
        }
        Ok(())
    }

    fn execute_transfers(&self, input: u64, output: u64, is_a_to_b: bool) -> Result<()> {
        if is_a_to_b {
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_token_a.to_account_info(),
                        to: self.token_a_reserve.to_account_info(),
                        authority: self.authority.to_account_info(),
                    },
                ),
                input,
            )?;
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_b_reserve.to_account_info(),
                        to: self.user_token_b.to_account_info(),
                        authority: self.pool.to_account_info(),
                    },
                ).with_signer(&[&[
                    b"pool",
                    self.token_a_reserve.mint.key().as_ref(),
                    self.token_b_reserve.mint.key().as_ref(),
                    &[self.pool.bump],
                ]]),
                output,
            )?;
        } else {
            // Similar logic for B to A swap
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_token_b.to_account_info(),
                        to: self.token_b_reserve.to_account_info(),
                        authority: self.authority.to_account_info(),
                    },
                ),
                input,
            )?;
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_a_reserve.to_account_info(),
                        to: self.user_token_a.to_account_info(),
                        authority: self.pool.to_account_info(),
                    },
                ).with_signer(&[&[
                    b"pool",
                    self.token_a_reserve.mint.key().as_ref(),
                    self.token_b_reserve.mint.key().as_ref(),
                    &[self.pool.bump],
                ]]),
                output,
            )?;
        }
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub lp_mint: Pubkey,
    pub total_lp_tokens: u64,
    pub fee_numerator: u64,
    pub fee_denominator: u64,
    pub bump: u8,
}

#[error_code]
pub enum AmmError {
    #[msg("The fee denominator must be greater than zero")]
    InvalidFee,

    #[msg("The provided amount must be greater than zero")]
    InvalidAmount,

    #[msg("Arithmetic error occurred during calculation")]
    ArithmeticError,

    #[msg("Swap would result in zero output amount")]
    ZeroSwapOutput,
    
    #[msg("Insufficient liquidity in pool")]
    InsufficientLiquidity,

    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
}