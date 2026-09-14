pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF");

#[program]
pub mod amm_q3_w3 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.handle_initialize(ctx.bumps.pool);
        Ok(())
    }
}
