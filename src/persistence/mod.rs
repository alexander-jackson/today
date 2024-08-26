use chrono::{NaiveDate, NaiveDateTime, Utc};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

pub mod account;
pub mod bootstrap;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemState {
    Checked,
    Unchecked,
    Deleted,
}

impl ItemState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Checked => "Checked",
            Self::Unchecked => "Unchecked",
            Self::Deleted => "Deleted",
        }
    }
}

impl From<String> for ItemState {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Checked" => Self::Checked,
            "Unchecked" => Self::Unchecked,
            "Deleted" => Self::Deleted,
            _ => panic!("invalid argument {value}"),
        }
    }
}

#[derive(Serialize)]
pub struct Item {
    item_uid: Uuid,
    content: String,
    pub state: ItemState,
}

pub async fn select_items(pool: &PgPool, date: NaiveDate) -> Result<Vec<Item>> {
    let items = sqlx::query_as!(
        Item,
        r#"
            WITH items_with_states AS (
                SELECT DISTINCT ON (i.id)
                    i.item_uid,
                    i.content,
                    iet.name AS state
                FROM item i
                JOIN item_event ie ON i.id = ie.item_id
                JOIN item_event_type iet ON iet.id = ie.event_type_id
                WHERE i.created_at::date = $1
                ORDER BY i.id, i.created_at, ie.occurred_at DESC
            )
            SELECT item_uid, content, state
            FROM items_with_states
            WHERE state != 'Deleted'
        "#,
        date,
    )
    .fetch_all(pool)
    .await?;

    Ok(items)
}

pub async fn create_item(
    pool: &PgPool,
    account_uid: Uuid,
    item_uid: Uuid,
    content: &str,
    created_at: NaiveDateTime,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
            INSERT INTO item (account_id, item_uid, content, created_at)
            VALUES ((SELECT id FROM account WHERE account_uid = $1), $2, $3, $4)
        "#,
        account_uid,
        item_uid,
        content,
        created_at,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
            INSERT INTO item_event (item_id, event_type_id, occurred_at)
            VALUES (
                (SELECT id FROM item WHERE item_uid = $1),
                (SELECT id FROM item_event_type WHERE name = 'Unchecked'),
                $2
            )
        "#,
        item_uid,
        created_at,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn update_item(pool: &PgPool, item_uid: Uuid, state: ItemState) -> Result<()> {
    let mut tx = pool.begin().await?;
    let now = Utc::now().naive_local();

    sqlx::query!(
        r#"
            INSERT INTO item_event (item_id, event_type_id, occurred_at)
            VALUES (
                (SELECT id FROM item WHERE item_uid = $1),
                (SELECT id FROM item_event_type WHERE name = $2),
                $3
            )
        "#,
        item_uid,
        state.as_str(),
        now,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests;
