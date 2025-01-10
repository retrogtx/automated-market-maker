use anchor_lang::prelude::*;

declare_id!("HVufAntkvawg85H1hdooyj1QHbY18V2QA7jGUzm7pKZ4");

#[program]
pub mod automated_market_maker {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
