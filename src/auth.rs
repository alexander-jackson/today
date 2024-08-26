use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::response::Redirect;
use axum_extra::extract::cookie::Key;
use axum_extra::extract::PrivateCookieJar;
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub account_uid: Uuid,
    exp: u32,
}

impl Account {
    pub fn new(account_uid: Uuid) -> Self {
        Self {
            account_uid,
            exp: u32::MAX,
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Account
where
    S: Send + Sync,
    Key: FromRef<S>,
    DecodingKey: FromRef<S>,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookies = PrivateCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .expect("Cookie jar creation cannot fail");

        let Some(cookie) = cookies.get("token") else {
            return Err(Redirect::to("/login"));
        };

        let decoding_key = DecodingKey::from_ref(state);
        let mut validation = Validation::default();

        // TODO: enable this in the future
        validation.validate_exp = false;

        let token: TokenData<Account> =
            jsonwebtoken::decode(cookie.value(), &decoding_key, &validation)
                .map_err(|_| Redirect::to("/login"))?;

        Ok(token.claims)
    }
}
