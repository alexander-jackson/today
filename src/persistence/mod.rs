use chrono::{NaiveDate, NaiveDateTime, Utc};
use color_eyre::eyre::Result;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub mod bootstrap;

#[derive(Serialize)]
pub struct Item {
    item_uid: Uuid,
    content: String,
    pub state: bool,
}

pub async fn select_items(pool: &PgPool, date: NaiveDate) -> Result<Vec<Item>> {
    let items = sqlx::query_as!(
        Item,
        r#"
            SELECT DISTINCT ON (i.id)
                i.item_uid,
                i.content,
                CASE WHEN iet.name = 'Checked' THEN true ELSE false END AS "state!"
            FROM item i
            JOIN item_event ie ON i.id = ie.item_id
            JOIN item_event_type iet ON iet.id = ie.event_type_id
            WHERE i.created_at::date = $1
            ORDER BY i.id, i.created_at, ie.occurred_at DESC
        "#,
        date,
    )
    .fetch_all(pool)
    .await?;

    Ok(items)
}

pub async fn create_item(
    pool: &PgPool,
    item_uid: Uuid,
    content: &str,
    created_at: NaiveDateTime,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
            INSERT INTO item (item_uid, content, created_at)
            VALUES ($1, $2, $3)
        "#,
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

pub async fn update_item(pool: &PgPool, item_uid: Uuid, state: bool) -> Result<()> {
    let mut tx = pool.begin().await?;
    let state = if state { "Checked" } else { "Unchecked" };

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
        state,
        now,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests;
