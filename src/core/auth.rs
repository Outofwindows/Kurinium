use std::sync::OnceLock;
use crate::prelude::*;
use crate::error::KuriniumError;

#[derive(Debug)]
pub struct AuthManager {
    config: crate::config::AuthConfig,
}

impl AuthManager {
    pub fn new(config: crate::config::AuthConfig) -> Self {
        Self { config }
    }

    pub async fn is_authorized_poise(&self, ctx: PoiseContext<'_>) -> KResult<bool> {
        if self.config.auth_all {
            return Ok(true);
        }

        let author_id = ctx.author().id.to_string();

        if self.config.auth_user {
            if self.config.allowed_users.contains(&author_id) {
                return Ok(true);
            }
        }

        if self.config.auth_roles {
            if let Some(member) = ctx.author_member().await {
                for role_id in &member.roles {
                    let role_str = role_id.to_string();
                    if self.config.allowed_roles.contains(&role_str) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub async fn get_auth_status_poise(&self, ctx: PoiseContext<'_>) -> String {
        if self.config.auth_all {
            return "**Authentication**: Disabled (everyone allowed)".to_string();
        }

        let mut status = String::new();

        if self.config.auth_roles {
            status.push_str("**Role auth**: Enabled\n");
            status.push_str(&format!(
                "   **Allowed roles**: {} roles\n",
                self.config.allowed_roles.len()
            ));
        }

        if self.config.auth_user {
            status.push_str("**User auth**: Enabled\n");
            status.push_str(&format!(
                "   **Allowed users**: {} users\n",
                self.config.allowed_users.len()
            ));
        }

        if let Ok(authorized) = self.is_authorized_poise(ctx).await {
            if authorized {
                status.push_str("**Your status**: Authorized");
            } else {
                status.push_str("**Your status**: Not authorized");
            }
        }

        status
    }

    pub fn get_config(&self) -> &crate::config::AuthConfig {
        &self.config
    }
}

static AUTH_MANAGER: OnceLock<AuthManager> = OnceLock::new();
pub fn get_auth_manager() -> KResult<&'static AuthManager> {
    AUTH_MANAGER.get().ok_or(KuriniumError::AuthNotInitialized)
}
pub fn get_auth_manager_unchecked() -> &'static AuthManager {
    AUTH_MANAGER
        .get()
        .expect("FATAL: Auth manager accessed before initialization - this is a bug")
}
pub fn init_auth_manager(config: crate::config::AuthConfig) -> KResult<()> {
    AUTH_MANAGER
        .set(AuthManager::new(config))
        .map_err(|_| KuriniumError::AuthAlreadyInitialized)
}
pub fn try_init_auth_manager(config: crate::config::AuthConfig) {
    let _ = AUTH_MANAGER.set(AuthManager::new(config));
}
