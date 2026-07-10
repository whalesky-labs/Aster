use std::collections::HashSet;

use crate::domain::reports::{ReportBundle, ReportBundlePage};
use crate::error::{AppError, AppResult};

const SECTIONS: [&str; 11] = [
    "monthlyInventory",
    "departmentSummary",
    "departmentDetails",
    "categoryConsumption",
    "itemConsumptionRanking",
    "inboundDetails",
    "outboundDetails",
    "salesProfit",
    "stockBalances",
    "stockWarnings",
    "stocktakeDifferences",
];

pub fn collect(
    month: &str,
    mut fetch: impl FnMut(&str, Option<&str>) -> AppResult<ReportBundlePage>,
) -> AppResult<ReportBundle> {
    let mut bundle = empty_bundle(month);
    for section in SECTIONS {
        collect_section(section, &mut bundle, &mut fetch)?;
    }
    Ok(bundle)
}

fn collect_section(
    section: &str,
    bundle: &mut ReportBundle,
    fetch: &mut impl FnMut(&str, Option<&str>) -> AppResult<ReportBundlePage>,
) -> AppResult<()> {
    let mut cursor = None;
    let mut seen = HashSet::new();
    loop {
        let page = fetch(section, cursor.as_deref())?;
        if page.section != section {
            return Err(AppError::Validation(
                "主机返回了错误的报表 section".to_string(),
            ));
        }
        merge(bundle, page.bundle);
        let Some(next) = page.next_cursor else {
            return Ok(());
        };
        if !seen.insert(next.clone()) {
            return Err(AppError::Validation(
                "主机返回了重复报表分页游标".to_string(),
            ));
        }
        cursor = Some(next);
    }
}

fn merge(target: &mut ReportBundle, mut page: ReportBundle) {
    target.monthly_inventory.append(&mut page.monthly_inventory);
    target
        .department_summary
        .append(&mut page.department_summary);
    target
        .department_details
        .append(&mut page.department_details);
    target
        .category_consumption
        .append(&mut page.category_consumption);
    target
        .item_consumption_ranking
        .append(&mut page.item_consumption_ranking);
    target.inbound_details.append(&mut page.inbound_details);
    target.outbound_details.append(&mut page.outbound_details);
    target.sales_profit.append(&mut page.sales_profit);
    target.stock_balances.append(&mut page.stock_balances);
    target.stock_warnings.append(&mut page.stock_warnings);
    target
        .stocktake_differences
        .append(&mut page.stocktake_differences);
}

fn empty_bundle(month: &str) -> ReportBundle {
    ReportBundle {
        month: month.to_string(),
        monthly_inventory: Vec::new(),
        department_summary: Vec::new(),
        department_details: Vec::new(),
        category_consumption: Vec::new(),
        item_consumption_ranking: Vec::new(),
        inbound_details: Vec::new(),
        outbound_details: Vec::new(),
        sales_profit: Vec::new(),
        stock_balances: Vec::new(),
        stock_warnings: Vec::new(),
        stocktake_differences: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_all_report_sections() {
        let bundle = collect("2026-06", |section, _| {
            Ok(ReportBundlePage {
                section: section.to_string(),
                bundle: empty_bundle("2026-06"),
                next_cursor: None,
            })
        })
        .expect("bundle");
        assert_eq!(bundle.month, "2026-06");
    }
}
