#![allow(dead_code)]

use color_eyre::eyre::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct EmailAddress(String);

impl<T> From<T> for EmailAddress
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        Self(value.as_ref().to_lowercase())
    }
}

#[derive(Clone, Debug)]
pub struct HashedPassword(String);

impl HashedPassword {
    pub fn from_raw<T: AsRef<str>>(value: T) -> Result<Self> {
        let hashed = bcrypt::hash(value.as_ref(), bcrypt::DEFAULT_COST)?;

        Ok(Self(hashed))
    }
}

pub struct Account {
    account_uid: Uuid,
    email_address: String,
    password: String,
}

pub async fn create_account(
    pool: &PgPool,
    account_uid: Uuid,
    email_address: &EmailAddress,
    password: &HashedPassword,
) -> Result<()> {
    sqlx::query!(
        r#"
            INSERT INTO account (account_uid, email_address, password, created_at)
            VALUES ($1, $2, $3, now()::timestamp)
        "#,
        account_uid,
        email_address.0,
        password.0
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fetch_account_by_email(
    pool: &PgPool,
    email_address: EmailAddress,
) -> Result<Option<Account>> {
    let account = sqlx::query_as!(
        Account,
        r#"
            SELECT
                account_uid,
                email_address,
                password
            FROM account
            WHERE email_address = $1
        "#,
        email_address.0
    )
    .fetch_optional(pool)
    .await?;

    Ok(account)
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::Result;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::{create_account, fetch_account_by_email, EmailAddress, HashedPassword};

    #[sqlx::test]
    async fn non_existent_email_addresses_return_no_results(pool: PgPool) -> Result<()> {
        let account = fetch_account_by_email(&pool, "example@domain.com".into()).await?;

        assert!(account.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn accounts_can_be_created(pool: PgPool) -> Result<()> {
        let account_uid = Uuid::new_v4();
        let email_address = EmailAddress::from("john@gmail.com");
        let password = HashedPassword::from_raw("password")?;

        // Insert the account
        create_account(&pool, account_uid, &email_address, &password).await?;

        // Validate we can fetch it by email again
        let account = fetch_account_by_email(&pool, email_address).await?;

        assert_eq!(account.map(|a| a.account_uid), Some(account_uid));

        Ok(())
    }

    #[sqlx::test]
    async fn cannot_create_multiple_accounts_with_same_email(pool: PgPool) -> Result<()> {
        let email_address = EmailAddress::from("john@gmail.com");
        let password = HashedPassword::from_raw("password")?;

        // Insert the account
        create_account(&pool, Uuid::new_v4(), &email_address, &password).await?;

        // Create another one with the same email
        let res = create_account(&pool, Uuid::new_v4(), &email_address, &password).await;

        let expected_error_message =
            r#"duplicate key value violates unique constraint "uk_account_email_address_lower""#;

        assert!(res.is_err_and(|e| e.to_string().contains(expected_error_message)));

        Ok(())
    }
}
