fn planned_outbound_costs(
    conn: &Connection,
    lines: &[DocumentLineForConfirm],
    allow_negative_stock: bool,
) -> AppResult<HashMap<String, f64>> {
    let mut costs = HashMap::new();
    let mut reserved_quantities: HashMap<String, f64> = HashMap::new();
    for line in lines {
        let allocations = allocate_fifo_batches_with_reservations(
            conn,
            &line.item_id,
            line.quantity,
            allow_negative_stock,
            &reserved_quantities,
        )?;
        let amount = if allocations.is_empty() {
            line.cost_amount.unwrap_or(line.amount)
        } else {
            round_money(allocations.iter().map(|item| item.amount).sum())
        };
        for allocation in &allocations {
            reserved_quantities
                .entry(allocation.batch_id.clone())
                .and_modify(|quantity| {
                    *quantity = round_quantity(*quantity + allocation.quantity);
                })
                .or_insert(allocation.quantity);
        }
        costs
            .entry(line.line_id.clone())
            .and_modify(|current| *current = round_money(*current + amount))
            .or_insert(amount);
    }
    Ok(costs)
}

fn allocate_fifo_batches(
    conn: &Connection,
    item_id: &str,
    quantity: f64,
    allow_negative_stock: bool,
) -> AppResult<Vec<BatchAllocation>> {
    allocate_fifo_batches_with_reservations(
        conn,
        item_id,
        quantity,
        allow_negative_stock,
        &HashMap::new(),
    )
}

fn allocate_fifo_batches_with_reservations(
    conn: &Connection,
    item_id: &str,
    quantity: f64,
    allow_negative_stock: bool,
    reserved_quantities: &HashMap<String, f64>,
) -> AppResult<Vec<BatchAllocation>> {
    ensure_opening_batch_from_balance(conn, item_id)?;
    let mut remaining = quantity;
    let mut allocations = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, remaining_quantity, remaining_amount, unit_price
         FROM stock_batches
         WHERE item_id = ?1
           AND status = 'available'
           AND remaining_quantity > 0
         ORDER BY inbound_date ASC, created_at ASC, batch_no ASC",
    )?;
    let rows = stmt.query_map(params![item_id], |row| {
        Ok(AvailableBatch {
            id: row.get(0)?,
            remaining_quantity: row.get(1)?,
            remaining_amount: row.get(2)?,
            unit_price: row.get(3)?,
        })
    })?;
    for batch in collect_rows(rows)? {
        if remaining <= 0.000001 {
            break;
        }
        let reserved_quantity = reserved_quantities.get(&batch.id).copied().unwrap_or(0.0);
        let available_quantity = round_quantity(batch.remaining_quantity - reserved_quantity);
        if available_quantity <= 0.000001 {
            continue;
        }
        let used_quantity = remaining.min(available_quantity);
        let available_amount = if available_quantity + 0.000001 >= batch.remaining_quantity {
            batch.remaining_amount
        } else {
            round_money(available_quantity * batch.unit_price)
        };
        let amount = if used_quantity + 0.000001 >= available_quantity {
            available_amount
        } else {
            round_money(used_quantity * batch.unit_price)
        };
        allocations.push(BatchAllocation {
            batch_id: batch.id,
            quantity: round_quantity(used_quantity),
            unit_price: batch.unit_price,
            amount,
            remaining_quantity: batch.remaining_quantity,
            remaining_amount: batch.remaining_amount,
        });
        remaining = round_quantity(remaining - used_quantity);
    }
    if remaining > 0.000001 && !allow_negative_stock {
        return Err(AppError::Validation(format!(
            "库存不足：当前可用批次数量 {:.2}，出库数量 {:.2}",
            quantity - remaining,
            quantity
        )));
    }
    Ok(allocations)
}

fn ensure_opening_batch_from_balance(conn: &Connection, item_id: &str) -> AppResult<()> {
    let existing_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_batches WHERE item_id = ?1",
        params![item_id],
        |row| row.get(0),
    )?;
    if existing_count > 0 {
        return Ok(());
    }
    let Some((item_code, quantity, amount, average_price, default_price)) = conn
        .query_row(
            "SELECT i.code, b.quantity, b.amount, b.average_price, i.default_price
             FROM stock_balances b
             JOIN master_items i ON i.id = b.item_id
             WHERE b.item_id = ?1
               AND b.quantity > 0",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(());
    };
    let unit_price = if average_price > 0.0 {
        average_price
    } else if quantity > 0.0 {
        round_price(amount / quantity)
    } else {
        default_price
    };
    let opening_amount = if amount > 0.0 {
        amount
    } else {
        round_money(quantity * unit_price)
    };
    conn.execute(
        "INSERT INTO stock_batches (
           id, item_id, source_document_id, source_document_line_id,
           batch_no, inbound_date, supplier_id, supplier_name,
           original_quantity, remaining_quantity, unit_price,
           original_amount, remaining_amount, status
         )
         VALUES (?1, ?2, NULL, NULL, ?3, '1970-01-01', NULL, '期初库存',
                 ?4, ?4, ?5, ?6, ?6, 'available')",
        params![
            new_id(),
            item_id,
            format!("OPEN-{item_code}"),
            quantity,
            unit_price,
            opening_amount
        ],
    )?;
    Ok(())
}

fn sync_balance_from_batches(conn: &Connection, item_id: &str) -> AppResult<()> {
    let (quantity, amount, last_inbound_price): (f64, f64, f64) = conn.query_row(
        "SELECT
           COALESCE(SUM(CASE WHEN status != 'voided' THEN remaining_quantity ELSE 0 END), 0),
           COALESCE(SUM(CASE WHEN status != 'voided' THEN remaining_amount ELSE 0 END), 0),
           COALESCE((
             SELECT unit_price
             FROM stock_batches latest
             WHERE latest.item_id = ?1
               AND latest.original_quantity > 0
             ORDER BY latest.inbound_date DESC, latest.created_at DESC
             LIMIT 1
           ), 0)
         FROM stock_batches
         WHERE item_id = ?1",
        params![item_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let average_price = if quantity.abs() < f64::EPSILON {
        0.0
    } else {
        round_price(amount / quantity)
    };
    conn.execute(
        "INSERT INTO stock_balances (
           id, item_id, quantity, amount, average_price, last_inbound_price, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
         ON CONFLICT(item_id) DO UPDATE SET
           quantity = excluded.quantity,
           amount = excluded.amount,
           average_price = excluded.average_price,
           last_inbound_price = excluded.last_inbound_price,
           updated_at = CURRENT_TIMESTAMP",
        params![
            new_id(),
            item_id,
            round_quantity(quantity),
            round_money(amount),
            average_price,
            last_inbound_price
        ],
    )?;
    Ok(())
}

fn document_has_batch_movements(conn: &Connection, document_id: &str) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_batch_movements WHERE document_id = ?1",
        params![document_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn reverse_batch_document(
    conn: &Connection,
    document_id: &str,
    document_no: &str,
    business_date: &str,
    reason: &str,
    handler: Option<String>,
) -> AppResult<()> {
    let batch_movements = load_batch_movements_for_document(conn, document_id)?;
    let operator = blank_to_none(handler).unwrap_or_else(|| "system".to_string());
    let mut touched_items: Vec<String> = Vec::new();
    for movement in batch_movements {
        let reverse_direction = if movement.direction == "in" {
            "out"
        } else {
            "in"
        };
        if movement.direction == "in" {
            let (original_quantity, remaining_quantity): (f64, f64) = conn.query_row(
                "SELECT original_quantity, remaining_quantity FROM stock_batches WHERE id = ?1",
                params![movement.batch_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if remaining_quantity + 0.000001 < original_quantity {
                return Err(AppError::Validation(format!(
                    "入库批次已被后续出库消耗，不能直接作废：{}",
                    movement.batch_no
                )));
            }
            conn.execute(
                "UPDATE stock_batches
                 SET remaining_quantity = 0,
                     remaining_amount = 0,
                     status = 'voided',
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![movement.batch_id],
            )?;
        } else {
            let next_quantity =
                round_quantity(movement.batch_remaining_quantity + movement.quantity);
            let next_amount = round_money(movement.batch_remaining_amount + movement.amount);
            conn.execute(
                "UPDATE stock_batches
                 SET remaining_quantity = ?1,
                     remaining_amount = ?2,
                     status = 'available',
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?3",
                params![next_quantity, next_amount, movement.batch_id],
            )?;
        }
        let reversal_movement_id = new_id();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, batch_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, department_id, department_name,
               supplier_id, supplier_name, movement_type,
               operator, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'reversal', ?15, ?16)",
            params![
                reversal_movement_id,
                business_date,
                movement.item_id,
                movement.batch_id,
                reverse_direction,
                movement.quantity,
                movement.unit_price,
                movement.amount,
                document_id,
                movement.document_line_id,
                movement.department_id,
                movement.department_name,
                movement.supplier_id,
                movement.supplier_name,
                operator,
                format!("作废冲正 {}：{}", document_no, reason)
            ],
        )?;
        conn.execute(
            "INSERT INTO stock_batch_movements (
               id, batch_id, stock_movement_id, document_id, document_line_id,
               direction, quantity, unit_price, amount, movement_type
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'reversal')",
            params![
                new_id(),
                movement.batch_id,
                reversal_movement_id,
                document_id,
                movement.document_line_id,
                reverse_direction,
                movement.quantity,
                movement.unit_price,
                movement.amount
            ],
        )?;
        if !touched_items.iter().any(|item| item == &movement.item_id) {
            touched_items.push(movement.item_id);
        }
    }
    for item_id in touched_items {
        sync_balance_from_batches(conn, &item_id)?;
    }
    Ok(())
}

fn load_document_movements(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<DocumentMovement>> {
    let mut stmt = conn.prepare(
        "SELECT item_id, direction, quantity, unit_price, amount,
                department_id, department_name, supplier_id, supplier_name
         FROM stock_movements
         WHERE document_id = ?1 AND movement_type != 'reversal'
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(DocumentMovement {
            item_id: row.get(0)?,
            direction: row.get(1)?,
            quantity: row.get(2)?,
            unit_price: row.get(3)?,
            amount: row.get(4)?,
            department_id: row.get(5)?,
            department_name: row.get(6)?,
            supplier_id: row.get(7)?,
            supplier_name: row.get(8)?,
        })
    })?;
    collect_rows(rows)
}

fn load_batch_movements_for_document(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<DocumentBatchMovement>> {
    let mut stmt = conn.prepare(
        "SELECT bm.batch_id, b.batch_no, b.item_id,
                bm.document_line_id, bm.direction, bm.quantity, bm.unit_price, bm.amount,
                COALESCE(m.department_id, NULL), m.department_name,
                COALESCE(m.supplier_id, NULL), m.supplier_name,
                b.remaining_quantity, b.remaining_amount
         FROM stock_batch_movements bm
         JOIN stock_batches b ON b.id = bm.batch_id
         LEFT JOIN stock_movements m ON m.id = bm.stock_movement_id
         WHERE bm.document_id = ?1
           AND bm.movement_type != 'reversal'
         ORDER BY bm.created_at DESC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(DocumentBatchMovement {
            batch_id: row.get(0)?,
            batch_no: row.get(1)?,
            item_id: row.get(2)?,
            document_line_id: row.get(3)?,
            direction: row.get(4)?,
            quantity: row.get(5)?,
            unit_price: row.get(6)?,
            amount: row.get(7)?,
            department_id: row.get(8)?,
            department_name: row.get(9)?,
            supplier_id: row.get(10)?,
            supplier_name: row.get(11)?,
            batch_remaining_quantity: row.get(12)?,
            batch_remaining_amount: row.get(13)?,
        })
    })?;
    collect_rows(rows)
}
