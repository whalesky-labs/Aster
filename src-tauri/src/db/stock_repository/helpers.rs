struct ItemForStock {
    id: String,
    default_price: f64,
    enabled: bool,
    category_id: Option<String>,
    category_name: Option<String>,
}

struct DocumentMovement {
    item_id: String,
    direction: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    department_id: Option<String>,
    department_name: Option<String>,
    supplier_id: Option<String>,
    supplier_name: Option<String>,
}

struct DocumentBatchMovement {
    batch_id: String,
    batch_no: String,
    item_id: String,
    document_line_id: Option<String>,
    direction: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    department_id: Option<String>,
    department_name: Option<String>,
    supplier_id: Option<String>,
    supplier_name: Option<String>,
    batch_remaining_quantity: f64,
    batch_remaining_amount: f64,
}

struct DocumentLineForConfirm {
    line_id: String,
    item_id: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    purchase_unit_price: Option<f64>,
    purchase_amount: Option<f64>,
    cost_amount: Option<f64>,
    remark: Option<String>,
}

struct AvailableBatch {
    id: String,
    remaining_quantity: f64,
    remaining_amount: f64,
    unit_price: f64,
}

struct BatchAllocation {
    batch_id: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    remaining_quantity: f64,
    remaining_amount: f64,
}

struct SnapshotNames {
    department_name: Option<String>,
    supplier_name: Option<String>,
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn snapshot_department_name(
    conn: &Connection,
    department_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(department_id) = department_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT name FROM departments WHERE id = ?1",
        params![department_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn snapshot_supplier_name(
    conn: &Connection,
    supplier_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(supplier_id) = supplier_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT name FROM suppliers WHERE id = ?1",
        params![supplier_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_quantity(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn round_price(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

struct LinePricing {
    unit_price: f64,
    amount: f64,
    purchase_unit_price: Option<f64>,
    purchase_amount: Option<f64>,
    sale_unit_price: Option<f64>,
    sale_amount: Option<f64>,
    cost_unit_price: Option<f64>,
    cost_amount: Option<f64>,
}

fn line_pricing(
    document_type: &str,
    outbound_kind: Option<&str>,
    line: &SubmitStockDocumentLine,
) -> LinePricing {
    let base_amount = line_amount(line);
    match document_type {
        "inbound" => {
            let purchase_unit_price = line.unit_price;
            let purchase_amount = effective_amount(line.quantity, purchase_unit_price, line.amount);
            LinePricing {
                unit_price: purchase_unit_price,
                amount: purchase_amount,
                purchase_unit_price: Some(purchase_unit_price),
                purchase_amount: Some(purchase_amount),
                sale_unit_price: None,
                sale_amount: None,
                cost_unit_price: Some(purchase_unit_price),
                cost_amount: Some(purchase_amount),
            }
        }
        "outbound" if outbound_kind == Some("guest_sale") => {
            let sale_unit_price = line.unit_price;
            let sale_amount = effective_amount(line.quantity, sale_unit_price, line.amount);
            LinePricing {
                unit_price: sale_unit_price,
                amount: sale_amount,
                purchase_unit_price: None,
                purchase_amount: None,
                sale_unit_price: Some(sale_unit_price),
                sale_amount: Some(sale_amount),
                cost_unit_price: None,
                cost_amount: None,
            }
        }
        "outbound" => LinePricing {
            unit_price: line.unit_price,
            amount: base_amount,
            purchase_unit_price: None,
            purchase_amount: None,
            sale_unit_price: None,
            sale_amount: None,
            cost_unit_price: Some(line.unit_price),
            cost_amount: Some(base_amount),
        },
        _ => LinePricing {
            unit_price: line.unit_price,
            amount: base_amount,
            purchase_unit_price: None,
            purchase_amount: None,
            sale_unit_price: None,
            sale_amount: None,
            cost_unit_price: Some(line.unit_price),
            cost_amount: Some(base_amount),
        },
    }
}

fn line_amount(line: &SubmitStockDocumentLine) -> f64 {
    effective_amount(line.quantity, line.unit_price, line.amount)
}

fn adjustment_line_amount(line: &crate::domain::stock::SubmitAdjustmentLine) -> f64 {
    effective_amount(line.quantity, line.unit_price, line.amount)
}

fn effective_amount(quantity: f64, unit_price: f64, amount: Option<f64>) -> f64 {
    round_money(
        amount
            .filter(|value| *value > 0.0)
            .unwrap_or(quantity * unit_price),
    )
}
