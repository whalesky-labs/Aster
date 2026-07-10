use std::collections::HashSet;

use crate::domain::pagination::Page;
use crate::error::{AppError, AppResult};

pub fn collect_all<T>(
    mut fetch: impl FnMut(Option<&str>) -> AppResult<Page<T>>,
) -> AppResult<Vec<T>> {
    let mut items = Vec::new();
    let mut cursor = None;
    let mut seen = HashSet::new();
    loop {
        let page = fetch(cursor.as_deref())?;
        items.extend(page.items);
        let Some(next_cursor) = page.next_cursor else {
            return Ok(items);
        };
        if !seen.insert(next_cursor.clone()) {
            return Err(AppError::Validation("主机返回了重复分页游标".to_string()));
        }
        cursor = Some(next_cursor);
    }
}

#[cfg(test)]
mod tests {
    use super::collect_all;
    use crate::domain::pagination::Page;

    #[test]
    fn aggregates_pages_without_exposing_cursor_to_callers() {
        let result = collect_all(|cursor| match cursor {
            None => Ok(Page {
                items: vec![1, 2],
                next_cursor: Some("next".to_string()),
            }),
            Some("next") => Ok(Page {
                items: vec![3],
                next_cursor: None,
            }),
            _ => unreachable!(),
        })
        .expect("collect pages");
        assert_eq!(result, vec![1, 2, 3]);
    }
}
