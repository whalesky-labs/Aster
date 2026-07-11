fn apply_inbound_line(
    conn: &Connection,
    document_id: &str,
    document_no: &str,
    request: &SubmitStockDocumentRequest,
    line: &DocumentLineForConfirm,
    supplier_id: Option<String>,
    supplier_name: Option<String>,
) -> AppResult<()> {
    create_batch_in_movement(
        conn,
        BatchInMovementInput {
            document_id,
            document_line_id: &line.line_id,
            document_no,
            item_id: &line.item_id,
            business_date: &request.business_date,
            quantity: line.quantity,
            unit_price: line.purchase_unit_price.unwrap_or(line.unit_price),
            amount: line.purchase_amount.unwrap_or(line.amount),
            supplier_id,
            supplier_name,
            movement_type: "inbound",
            operator: blank_to_none(request.handler.clone())
                .unwrap_or_else(|| "system".to_string()),
            remark: blank_to_none(line.remark.clone()),
        },
    )
}

struct OutboundLineInput<'a> {
    document_id: &'a str,
    request: &'a SubmitStockDocumentRequest,
    line: &'a DocumentLineForConfirm,
    department_id: Option<String>,
    department_name: Option<String>,
    supplier_id: Option<String>,
    supplier_name: Option<String>,
    allow_negative_stock: bool,
    default_price: f64,
}

fn apply_outbound_line(conn: &Connection, input: OutboundLineInput<'_>) -> AppResult<()> {
    let (actual_unit_price, actual_amount) = create_batch_out_movements(
        conn,
        BatchOutMovementInput {
            document_id: input.document_id,
            document_line_id: &input.line.line_id,
            item_id: &input.line.item_id,
            business_date: &input.request.business_date,
            quantity: input.line.quantity,
            department_id: input.department_id,
            department_name: input.department_name,
            supplier_id: input.supplier_id,
            supplier_name: input.supplier_name,
            movement_type: "outbound",
            operator: blank_to_none(input.request.handler.clone())
                .unwrap_or_else(|| "system".to_string()),
            remark: blank_to_none(input.line.remark.clone()),
            allow_negative_stock: input.allow_negative_stock,
            fallback_unit_price: input.line.unit_price.max(input.default_price),
        },
    )?;
    conn.execute(
        "UPDATE stock_document_lines
         SET unit_price = ?1, amount = ?2,
             cost_unit_price = ?1, cost_amount = ?2
         WHERE id = ?3",
        params![actual_unit_price, actual_amount, input.line.line_id],
    )?;
    Ok(())
}

pub(crate) struct BatchInMovementInput<'a> {
    pub(crate) document_id: &'a str,
    pub(crate) document_line_id: &'a str,
    pub(crate) document_no: &'a str,
    pub(crate) item_id: &'a str,
    pub(crate) business_date: &'a str,
    pub(crate) quantity: f64,
    pub(crate) unit_price: f64,
    pub(crate) amount: f64,
    pub(crate) supplier_id: Option<String>,
    pub(crate) supplier_name: Option<String>,
    pub(crate) movement_type: &'a str,
    pub(crate) operator: String,
    pub(crate) remark: Option<String>,
}

pub(crate) fn create_batch_in_movement(
    conn: &Connection,
    input: BatchInMovementInput<'_>,
) -> AppResult<()> {
    ensure_opening_batch_from_balance(conn, input.item_id)?;
    let batch_id = new_id();
    let batch_no = next_batch_no(conn, input.document_no)?;
    let unit_price = if input.quantity.abs() < f64::EPSILON || input.amount <= 0.0 {
        input.unit_price
    } else {
        round_price(input.amount / input.quantity)
    };
    conn.execute(
        "INSERT INTO stock_batches (
           id, item_id, source_document_id, source_document_line_id,
           batch_no, inbound_date, supplier_id, supplier_name,
           original_quantity, remaining_quantity, unit_price,
           original_amount, remaining_amount, status
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10, ?11, ?11, 'available')",
        params![
            batch_id,
            input.item_id,
            input.document_id,
            input.document_line_id,
            batch_no,
            input.business_date,
            input.supplier_id.clone(),
            input.supplier_name.clone(),
            input.quantity,
            unit_price,
            input.amount
        ],
    )?;
    let movement_id = new_id();
    conn.execute(
        "INSERT INTO stock_movements (
           id, movement_date, item_id, batch_id, direction, quantity, unit_price, amount,
           document_id, document_line_id, supplier_id, supplier_name, movement_type,
           operator, remark
         )
         VALUES (?1, ?2, ?3, ?4, 'in', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            movement_id,
            input.business_date,
            input.item_id,
            batch_id,
            input.quantity,
            unit_price,
            input.amount,
            input.document_id,
            input.document_line_id,
            input.supplier_id,
            input.supplier_name,
            input.movement_type,
            input.operator,
            input.remark
        ],
    )?;
    conn.execute(
        "INSERT INTO stock_batch_movements (
           id, batch_id, stock_movement_id, document_id, document_line_id,
           direction, quantity, unit_price, amount, movement_type
         )
         VALUES (?1, ?2, ?3, ?4, ?5, 'in', ?6, ?7, ?8, ?9)",
        params![
            new_id(),
            batch_id,
            movement_id,
            input.document_id,
            input.document_line_id,
            input.quantity,
            unit_price,
            input.amount,
            input.movement_type
        ],
    )?;
    sync_balance_from_batches(conn, input.item_id)
}

pub(crate) struct BatchOutMovementInput<'a> {
    pub(crate) document_id: &'a str,
    pub(crate) document_line_id: &'a str,
    pub(crate) item_id: &'a str,
    pub(crate) business_date: &'a str,
    pub(crate) quantity: f64,
    pub(crate) department_id: Option<String>,
    pub(crate) department_name: Option<String>,
    pub(crate) supplier_id: Option<String>,
    pub(crate) supplier_name: Option<String>,
    pub(crate) movement_type: &'a str,
    pub(crate) operator: String,
    pub(crate) remark: Option<String>,
    pub(crate) allow_negative_stock: bool,
    pub(crate) fallback_unit_price: f64,
}

pub(crate) fn create_batch_out_movements(
    conn: &Connection,
    input: BatchOutMovementInput<'_>,
) -> AppResult<(f64, f64)> {
    let allocations = allocate_fifo_batches(
        conn,
        input.item_id,
        input.quantity,
        input.allow_negative_stock,
    )?;
    let allocated_quantity = round_quantity(allocations.iter().map(|item| item.quantity).sum());
    let mut actual_amount = round_money(allocations.iter().map(|item| item.amount).sum());

    for allocation in allocations {
        let remaining_quantity =
            round_quantity(allocation.remaining_quantity - allocation.quantity);
        let remaining_amount = round_money(allocation.remaining_amount - allocation.amount);
        let status = if remaining_quantity.abs() < 0.000001 {
            "depleted"
        } else {
            "available"
        };
        conn.execute(
            "UPDATE stock_batches
             SET remaining_quantity = ?1,
                 remaining_amount = ?2,
                 status = ?3,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![
                remaining_quantity.max(0.0),
                remaining_amount.max(0.0),
                status,
                allocation.batch_id
            ],
        )?;
        let movement_id = new_id();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, batch_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, department_id, department_name,
               supplier_id, supplier_name, movement_type,
               operator, remark
             )
             VALUES (?1, ?2, ?3, ?4, 'out', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                movement_id,
                input.business_date,
                input.item_id,
                allocation.batch_id,
                allocation.quantity,
                allocation.unit_price,
                allocation.amount,
                input.document_id,
                input.document_line_id,
                input.department_id.clone(),
                input.department_name.clone(),
                input.supplier_id.clone(),
                input.supplier_name.clone(),
                input.movement_type,
                input.operator.clone(),
                input.remark.clone()
            ],
        )?;
        conn.execute(
            "INSERT INTO stock_batch_movements (
               id, batch_id, stock_movement_id, document_id, document_line_id,
               direction, quantity, unit_price, amount, movement_type
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'out', ?6, ?7, ?8, ?9)",
            params![
                new_id(),
                allocation.batch_id,
                movement_id,
                input.document_id,
                input.document_line_id,
                allocation.quantity,
                allocation.unit_price,
                allocation.amount,
                input.movement_type
            ],
        )?;
    }

    let short_quantity = round_quantity(input.quantity - allocated_quantity);
    if short_quantity > 0.000001 && input.allow_negative_stock {
        sync_balance_from_batches(conn, input.item_id)?;
        let short_amount = round_money(short_quantity * input.fallback_unit_price);
        actual_amount = round_money(actual_amount + short_amount);
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, department_id, department_name,
               supplier_id, supplier_name, movement_type,
               operator, remark
             )
             VALUES (?1, ?2, ?3, 'out', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                new_id(),
                input.business_date,
                input.item_id,
                short_quantity,
                input.fallback_unit_price,
                short_amount,
                input.document_id,
                input.document_line_id,
                input.department_id,
                input.department_name,
                input.supplier_id,
                input.supplier_name,
                input.movement_type,
                input.operator,
                input.remark
            ],
        )?;
        crate::db::balance_repository::apply(
            conn,
            crate::db::balance_repository::BalanceChange {
                item_id: input.item_id,
                direction: "out",
                quantity: short_quantity,
                unit_price: input.fallback_unit_price,
                amount: short_amount,
                default_price: input.fallback_unit_price,
                allow_negative_stock: true,
            },
        )?;
    } else {
        sync_balance_from_batches(conn, input.item_id)?;
    }

    let actual_unit_price = if input.quantity.abs() < f64::EPSILON {
        0.0
    } else {
        round_price(actual_amount / input.quantity)
    };
    Ok((actual_unit_price, actual_amount))
}
