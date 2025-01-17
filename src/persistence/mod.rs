use chrono::{NaiveDate, NaiveDateTime, Utc};
use color_eyre::eyre::Result;
use pulldown_cmark::{Event, Parser};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Content(String);

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut rendered = String::new();

        for event in Parser::new(&self.0) {
            match event {
                Event::Text(value) => rendered.push_str(&value),
                Event::Code(value) => {
                    rendered.push_str("<code>");
                    rendered.push_str(&value);
                    rendered.push_str("</code>");
                }
                _ => {}
            };
        }

        serializer.serialize_str(&rendered)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Item {
    pub item_uid: Uuid,
    pub content: Content,
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
                AND i.created_at::date = $1
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

pub async fn update_item(pool: &PgPool, item_uid: Uuid, state: ItemState) -> Result<()> {
    let mut tx = pool.begin().await?;
    let now = Utc::now().naive_local();

    sqlx::query!(
        r#"
            INSERT INTO item_event (item_id, event_type_id, occurred_at)
            VALUES (
                (
                    SELECT i.id
                    FROM item i
                    WHERE i.item_uid = $1
                ),
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
