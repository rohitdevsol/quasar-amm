use quasar_lang::prelude::*;

use crate::{ errors::AmmError, events::PoolLockToggled, state::Config };

#[derive(Accounts)]
pub struct TogglePool<'info> {
    // admin who has authority to lock/unlock the pool
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
    /// Toggles the pool lock state between locked and unlocked
    /// Only the designated authority can call this
    pub fn toggle_pool(&mut self) -> Result<(), ProgramError> {
        // ensure the pool has an authority set
        require!(self.config.authority.is_some(), AmmError::InvalidAuthority);

        // check if the signer matches the pool authority
        let owner_matched = match self.config.authority {
            Some(authority) => *self.admin.address() == authority,
            _ => false,
        };

        // reject if the signer is not the authority
        if !owner_matched {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // means the owner is trying to modify this .. allow
        self.config.locked = !self.config.locked;

        // emit event so indexers can track pool lock state changes
        emit!(PoolLockToggled {
            authority: *self.admin.address(),
            locked: self.config.locked.into(),
            config: *self.config.address(),
        });

        Ok(())
    }
}
