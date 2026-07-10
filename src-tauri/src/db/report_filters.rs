use crate::domain::reports::ReportQuery;

pub struct ReportFilters<'a> {
    pub month: &'a str,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub department_id: Option<&'a str>,
    pub category_id: Option<&'a str>,
    pub item_id: Option<&'a str>,
    pub supplier_id: Option<&'a str>,
}

impl<'a> From<&'a ReportQuery> for ReportFilters<'a> {
    fn from(query: &'a ReportQuery) -> Self {
        Self {
            month: &query.month,
            start_date: query.start_date.as_deref().map(|value| bound(value, false)),
            end_date: query.end_date.as_deref().map(|value| bound(value, true)),
            department_id: query.department_id.as_deref(),
            category_id: query.category_id.as_deref(),
            item_id: query.item_id.as_deref(),
            supplier_id: query.supplier_id.as_deref(),
        }
    }
}

fn bound(value: &str, end: bool) -> String {
    if value.len() == 10 {
        format!("{value} {}", if end { "23:59:59" } else { "00:00:00" })
    } else {
        value.to_string()
    }
}
