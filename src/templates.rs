use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use color_eyre::eyre::Result;
use serde::Serialize;
use tera::{Context, Tera};

use crate::persistence::{Item, ItemState};

#[derive(Clone, Serialize)]
pub struct IndexContext {
    checked_items: Vec<Item>,
    unchecked_items: Vec<Item>,
}

impl From<Vec<Item>> for IndexContext {
    fn from(items: Vec<Item>) -> Self {
        let mut checked_items = Vec::new();
        let mut unchecked_items = Vec::new();

        for item in items {
            match item.state {
                ItemState::Checked => checked_items.push(item),
                ItemState::Unchecked => unchecked_items.push(item),
                ItemState::Deleted => (), // intentionally ignored
            }
        }

        Self {
            checked_items,
            unchecked_items,
        }
    }
}

#[derive(Clone)]
pub struct TemplateEngine {
    inner: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let inner = Tera::new("templates/**.tera.html")?;

        Ok(Self { inner })
    }

    pub fn render_serialized<C: Serialize>(
        &self,
        template: &str,
        context: &C,
    ) -> Result<RenderedTemplate> {
        let context = Context::from_serialize(context)?;
        let rendered = self.inner.render(template, &context)?;

        Ok(RenderedTemplate { inner: rendered })
    }
}

pub struct RenderedTemplate {
    inner: String,
}

impl IntoResponse for RenderedTemplate {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/html")
            .header(CACHE_CONTROL, "no-store")
            .body(Body::from(self.inner))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::persistence::{Item, ItemState};
    use crate::templates::IndexContext;

    #[test]
    fn items_are_correctly_categorised() {
        let checked_item = Item {
            item_uid: Uuid::new_v4(),
            content: "checked".to_owned().into(),
            state: ItemState::Checked,
        };

        let unchecked_item = Item {
            item_uid: Uuid::new_v4(),
            content: "unchecked".to_owned().into(),
            state: ItemState::Unchecked,
        };

        let items = vec![checked_item.clone(), unchecked_item.clone()];
        let context = IndexContext::from(items);

        assert_eq!(context.checked_items, vec![checked_item]);
        assert_eq!(context.unchecked_items, vec![unchecked_item]);
    }

    #[test]
    fn deleted_items_are_ignored() {
        let deleted_item = Item {
            item_uid: Uuid::new_v4(),
            content: "deleted".to_owned().into(),
            state: ItemState::Deleted,
        };

        let items = vec![deleted_item.clone()];
        let context = IndexContext::from(items);

        assert!(
            !context.checked_items.contains(&deleted_item),
            "checked items contained a deleted item"
        );

        assert!(
            !context.unchecked_items.contains(&deleted_item),
            "unchecked items contained a delete item"
        );
    }
}
