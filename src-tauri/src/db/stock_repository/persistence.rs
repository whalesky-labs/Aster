fn insert_confirmed_document(
    tx: &Connection,
    document_id: &str,
    document_no: &str,
    request: SubmitStockDocumentRequest,
    allow_negative_stock: bool,
) -> AppResult<()> {
    let department_id = blank_to_none(request.department_id.clone());
    let supplier_id = blank_to_none(request.supplier_id.clone());
    let outbound_kind =
        normalized_outbound_kind(&request.document_type, request.outbound_kind.as_deref())?;
    let department_name = snapshot_department_name(tx, department_id.as_deref())?;
    let supplier_name = snapshot_supplier_name(tx, supplier_id.as_deref())?;
    tx.execute(
        "INSERT INTO stock_documents (
           id, document_no, document_type, outbound_kind, business_date, department_id,
           department_name, supplier_id, supplier_name, handler, purpose,
           approval_request_id, status, remark, confirmed_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'confirmed', ?13, CURRENT_TIMESTAMP)",
        params![
            document_id,
            document_no,
            request.document_type,
            outbound_kind,
            request.business_date,
            department_id,
            department_name,
            supplier_id,
            supplier_name,
            blank_to_none(request.handler.clone()),
            blank_to_none(request.purpose.clone()),
            blank_to_none(request.approval_request_id.clone()),
            blank_to_none(request.remark.clone())
        ],
    )?;
    for line in &request.lines {
        let pricing = line_pricing(&request.document_type, outbound_kind.as_deref(), line);
        tx.execute(
            "INSERT INTO stock_document_lines (
               id, document_id, item_id, quantity, unit_price, amount,
               purchase_unit_price, purchase_amount, sale_unit_price, sale_amount,
               cost_unit_price, cost_amount, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                new_id(),
                document_id,
                line.item_id,
                line.quantity,
                pricing.unit_price,
                pricing.amount,
                pricing.purchase_unit_price,
                pricing.purchase_amount,
                pricing.sale_unit_price,
                pricing.sale_amount,
                pricing.cost_unit_price,
                pricing.cost_amount,
                blank_to_none(line.remark.clone())
            ],
        )?;
    }
    apply_confirmed_document_effects(
        tx,
        document_id,
        document_no,
        request,
        SnapshotNames {
            department_name,
            supplier_name,
        },
        allow_negative_stock,
    )
}

fn apply_confirmed_document_effects(
    tx: &Connection,
    document_id: &str,
    document_no: &str,
    request: SubmitStockDocumentRequest,
    snapshot_names: SnapshotNames,
    allow_negative_stock: bool,
) -> AppResult<()> {
    validate_enabled_parties_for_document(tx, &request)?;
    let lines = load_document_lines_for_confirm(tx, document_id)?;
    let outbound_costs = if request.document_type == "outbound" {
        planned_outbound_costs(tx, &lines, allow_negative_stock)?
    } else {
        HashMap::new()
    };
    enforce_budget_limits(tx, &request, &lines, &outbound_costs)?;
    let department_id = blank_to_none(request.department_id.clone());
    let supplier_id = blank_to_none(request.supplier_id.clone());
    for line in lines {
        let item = get_item_for_stock(tx, &line.item_id)?;
        if request.document_type == "inbound" {
            apply_inbound_line(
                tx,
                document_id,
                document_no,
                &request,
                &line,
                supplier_id.clone(),
                snapshot_names.supplier_name.clone(),
            )?;
        } else {
            apply_outbound_line(
                tx,
                OutboundLineInput {
                    document_id,
                    request: &request,
                    line: &line,
                    department_id: department_id.clone(),
                    department_name: snapshot_names.department_name.clone(),
                    supplier_id: supplier_id.clone(),
                    supplier_name: snapshot_names.supplier_name.clone(),
                    allow_negative_stock,
                    default_price: item.default_price,
                },
            )?;
        }
    }
    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, ?2, 'stock_document', ?3, ?4, ?5)",
        params![
            new_id(),
            "submit_stock_document",
            document_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;
    Ok(())
}

fn load_document_lines_for_submit(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<SubmitStockDocumentLine>> {
    let mut stmt = conn.prepare(
        "SELECT item_id, quantity, unit_price, amount, remark
         FROM stock_document_lines
         WHERE document_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(SubmitStockDocumentLine {
            item_id: row.get(0)?,
            quantity: row.get(1)?,
            unit_price: row.get(2)?,
            amount: row.get(3)?,
            remark: row.get(4)?,
        })
    })?;
    collect_rows(rows)
}

fn load_document_lines_for_confirm(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<DocumentLineForConfirm>> {
    let mut stmt = conn.prepare(
        "SELECT id, item_id, quantity, unit_price, amount,
                purchase_unit_price, purchase_amount,
                cost_amount,
                remark
         FROM stock_document_lines
         WHERE document_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(DocumentLineForConfirm {
            line_id: row.get(0)?,
            item_id: row.get(1)?,
            quantity: row.get(2)?,
            unit_price: row.get(3)?,
            amount: row.get(4)?,
            purchase_unit_price: row.get(5)?,
            purchase_amount: row.get(6)?,
            cost_amount: row.get(7)?,
            remark: row.get(8)?,
        })
    })?;
    collect_rows(rows)
}
