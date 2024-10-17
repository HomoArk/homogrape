use crate::tg::types::LoginState;
use crate::tg::Backend;
use anyhow::Result;
use grammers_client::SignInError;
use log::{debug, error};

impl Backend {
    pub async fn login_with_phone(&mut self, phone: String) -> Result<LoginState> {
        if !self.is_logged_in().await {
            debug!("Signing in...");

            let login_token = self.client.request_login_code(&phone).await;
            match login_token {
                Ok(token) => {
                    self.login_token.replace(token);
                }
                Err(e) => {
                    error!("Failed to request login code: {e}");
                    return Err(anyhow::Error::from(e));
                }
            }
            self.login_state.replace(LoginState::CodeRequired);
            debug!("Waiting for code...");
        } else {
            debug!("Already signed in!");
            self.login_state.replace(LoginState::LoggedIn);
            self.save_session().await;
            return Ok(LoginState::LoggedIn);
        }

        Ok(LoginState::CodeRequired)
    }

    pub async fn provide_verify_code(&mut self, code: String) -> Result<LoginState> {
        if !self.is_logged_in().await {
            let signed_in = self
                .client
                .sign_in(self.login_token.as_ref().unwrap(), &code)
                .await;
            match signed_in {
                Err(SignInError::PasswordRequired(password_token)) => {
                    debug!("Password required");
                    self.login_state.replace(LoginState::PasswordRequired);
                    self.password_token.replace(password_token);
                    self.login_token.take();
                    return Ok(LoginState::PasswordRequired);
                }
                Ok(user) => {
                    self.user.replace(user);
                    debug!("Signed in!");
                    self.login_state.replace(LoginState::LoggedIn);
                    self.save_session().await;
                }
                Err(SignInError::InvalidCode) => {
                    debug!("Invalid code!");
                    self.login_state.replace(LoginState::WrongCode);
                    return Ok(LoginState::WrongCode);
                }
                Err(e) => {
                    error!("Failed to sign in: {e}");
                    return Err(anyhow::Error::from(e));
                }
            };
        } else {
            debug!("Already signed in!");
            self.login_state.replace(LoginState::LoggedIn);
            self.save_session().await;
        }

        Ok(LoginState::LoggedIn)
    }

    pub async fn provide_password(&mut self, password: String) -> Result<LoginState> {
        if self.login_state.as_ref().unwrap() != &LoginState::PasswordRequired {
            return Err(anyhow::anyhow!("Password not required!"));
        }
        if !self.is_logged_in().await {
            let signed_in = self
                .client
                .check_password(self.password_token.clone().unwrap(), &password)
                .await;
            match signed_in {
                Ok(user) => {
                    debug!("Signed in!");
                    self.user.replace(user);
                    self.login_state.replace(LoginState::LoggedIn);
                    self.password_token.take();
                }
                Err(e) => return Err(anyhow::Error::from(e)),
            };
        } else {
            debug!("Already signed in!");
            self.login_state.replace(LoginState::LoggedIn);
        }

        self.save_session().await;

        Ok(LoginState::LoggedIn)
    }
}