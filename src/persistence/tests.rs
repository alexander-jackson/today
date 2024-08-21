use std::collections::HashSet;

use chrono::{Duration, NaiveDateTime, Utc};
use color_eyre::eyre::Result;
use sqlx::PgPool;
use uuid::Uuid;

async fn create_item(pool: &PgPool, content: &str, created_at: NaiveDateTime) -> Result<Uuid> {
    let item_uid = Uuid::new_v4();

    super::create_item(pool, item_uid, content, created_at).await?;

    Ok(item_uid)
}

#[sqlx::test]
fn items_are_only_returned_for_the_current_day(pool: PgPool) -> Result<()> {
    let now = Utc::now().naive_local();
    let yesterday = now - Duration::days(1);

    // Insert an item for the current day
    let item_uid1 = create_item(&pool, "Today", now).await?;

    // Insert an item for yesterday
    let item_uid2 = create_item(&pool, "Yesterday", yesterday).await?;

    // Fetch all the items for today
    let items: HashSet<Uuid> = super::select_items(&pool, now)
        .await?
        .iter()
        .map(|item| item.item_uid)
        .collect();

    assert!(items.contains(&item_uid1));
    assert!(!items.contains(&item_uid2));

    Ok(())
}
