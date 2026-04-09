use quasar_lang::prelude::*;

use crate::{ errors::AmmError, events::PoolLockToggled, state::Config };

#[derive(Accounts)]
pub struct TogglePool<'info> {
    pub admin: &'info mut Signer,

    // main config of amm
    #[account(
        //seeds = [b"config"], // , config.seed.get() -- facing issues here
        seeds = Config::seeds(config.seed),
        bump = config.config_bump
    )]
    pub config: &'info mut Account<Config>,
}

impl<'info> TogglePool<'info> {
    pub fn toggle_pool(&mut self) -> Result<(), ProgramError> {
        require!(self.config.authority.is_some(), AmmError::InvalidAuthority);

        let owner_matched = match self.config.authority {
            Some(authority) => *self.admin.address() == authority,
            _ => false,
        };

        if !owner_matched {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // means the owner is trying to modify this .. allow
        self.config.locked = !self.config.locked;

        emit!(PoolLockToggled {
            authority: *self.admin.address(),
            locked: self.config.locked.into(),
            config: *self.config.address(),
        });

        Ok(())
    }
}
