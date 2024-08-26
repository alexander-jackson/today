use std::collections::HashSet;

use chrono::{Duration, NaiveDateTime, Utc};
use color_eyre::eyre::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::persistence::account::{EmailAddress, HashedPassword};
use crate::persistence::ItemState;

const SOME_EMAIL_ADDRESS: &str = "test@test.com";

async fn create_item(
    pool: &PgPool,
    account_uid: Uuid,
    content: &str,
    created_at: NaiveDateTime,
) -> Result<Uuid> {
    let item_uid = Uuid::new_v4();

    super::create_item(pool, account_uid, item_uid, content, created_at).await?;

    Ok(item_uid)
}

async fn create_account(pool: &PgPool, email_address: &str) -> Result<Uuid> {
    let account_uid = Uuid::new_v4();

    super::account::create_account(
        &pool,
        account_uid,
        &EmailAddress::from(email_address),
        &HashedPassword::from_raw("foobar")?,
    )
    .await?;

    Ok(account_uid)
}

#[sqlx::test]
fn items_are_only_returned_for_the_current_day(pool: PgPool) -> Result<()> {
    let now = Utc::now().naive_local();
    let yesterday = now - Duration::days(1);
    let today = now.date();

    let account_uid = create_account(&pool, SOME_EMAIL_ADDRESS).await?;

    // Insert an item for the current day
    let item_uid1 = create_item(&pool, account_uid, "Today", now).await?;

    // Insert an item for yesterday
    let item_uid2 = create_item(&pool, account_uid, "Yesterday", yesterday).await?;

    // Fetch all the items for today
    let items: HashSet<Uuid> = super::select_items(&pool, today)
        .await?
        .iter()
        .map(|item| item.item_uid)
        .collect();

    assert!(items.contains(&item_uid1));
    assert!(!items.contains(&item_uid2));

    Ok(())
}

#[sqlx::test]
fn item_states_can_be_modified(pool: PgPool) -> Result<()> {
    let now = Utc::now().naive_local();
    let today = now.date();

    let account_uid = create_account(&pool, SOME_EMAIL_ADDRESS).await?;

    // Insert an item to modify
    let item_uid = create_item(&pool, account_uid, "Content", now).await?;

    // Check it's currently unchecked
    let items = super::select_items(&pool, today).await?;
    assert_eq!(items[0].state, ItemState::Unchecked);

    // Modify the item
    super::update_item(&pool, item_uid, ItemState::Checked).await?;

    // Check the new state is reflected
    let items = super::select_items(&pool, today).await?;
    assert_eq!(items[0].state, ItemState::Checked);

    // Update it back to be unchecked
    super::update_item(&pool, item_uid, ItemState::Unchecked).await?;

    // Check it's back to what it was before
    let items = super::select_items(&pool, today).await?;
    assert_eq!(items[0].state, ItemState::Unchecked);

    Ok(())
}

#[sqlx::test]
fn deleted_items_do_not_get_returned(pool: PgPool) -> Result<()> {
    let now = Utc::now().naive_local();
    let today = now.date();

    let account_uid = create_account(&pool, SOME_EMAIL_ADDRESS).await?;

    // Create 2 items
    let item_uid1 = create_item(&pool, account_uid, "First", now).await?;
    let item_uid2 = create_item(&pool, account_uid, "Second", now).await?;

    // Mark the first as deleted
    super::update_item(&pool, item_uid1, ItemState::Deleted).await?;

    // Fetch all the items and assert we just have the second
    let items: HashSet<_> = super::select_items(&pool, today)
        .await?
        .iter()
        .map(|i| i.item_uid)
        .collect();

    assert!(!items.contains(&item_uid1));
    assert!(items.contains(&item_uid2));

    Ok(())
}
