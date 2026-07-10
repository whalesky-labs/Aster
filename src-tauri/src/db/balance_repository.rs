use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

pub struct BalanceChange<'a> {
    pub item_id: &'a str,
    pub direction: &'a str,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub default_price: f64,
    pub allow_negative_stock: bool,
}

pub fn apply(conn: &Connection, change: BalanceChange<'_>) -> AppResult<()> {
    let existing = conn
        .query_row(
            "SELECT quantity, amount, average_price, last_inbound_price
             FROM stock_balances WHERE item_id = ?1",
            params![change.item_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .unwrap_or((0.0, 0.0, change.default_price, 0.0));
    let next = calculate_balance(existing, &change)?;
    conn.execute(
        "INSERT INTO stock_balances (
           id, item_id, quantity, amount, average_price, last_inbound_price, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
         ON CONFLICT(item_id) DO UPDATE SET
           quantity = excluded.quantity, amount = excluded.amount,
           average_price = excluded.average_price,
           last_inbound_price = excluded.last_inbound_price,
           updated_at = CURRENT_TIMESTAMP",
        params![
            uuid::Uuid::new_v4().to_string(),
            change.item_id,
            next.0,
            next.1,
            next.2,
            next.3
        ],
    )?;
    Ok(())
}

fn calculate_balance(
    existing: (f64, f64, f64, f64),
    change: &BalanceChange<'_>,
) -> AppResult<(f64, f64, f64, f64)> {
    let (old_quantity, old_amount, old_average_price, old_last_price) = existing;
    if change.direction == "in" {
        let quantity = old_quantity + change.quantity;
        let amount = old_amount + change.amount;
        return Ok((
            quantity,
            round_money(amount),
            average_price(quantity, amount),
            change.unit_price,
        ));
    }
    if !change.allow_negative_stock && old_quantity + f64::EPSILON < change.quantity {
        return Err(AppError::Validation(format!(
            "库存不足：当前库存 {old_quantity}，出库数量 {}",
            change.quantity
        )));
    }
    let price = if old_average_price > 0.0 {
        old_average_price
    } else {
        change.unit_price
    };
    let outgoing_amount = if change.amount > 0.0 {
        change.amount
    } else {
        round_money(change.quantity * price)
    };
    let quantity = old_quantity - change.quantity;
    let amount = if change.allow_negative_stock {
        old_amount - outgoing_amount
    } else {
        (old_amount - outgoing_amount).max(0.0)
    };
    Ok((
        quantity,
        round_money(amount),
        average_price(quantity, amount),
        old_last_price,
    ))
}

fn average_price(quantity: f64, amount: f64) -> f64 {
    if quantity.abs() < f64::EPSILON {
        0.0
    } else {
        (amount / quantity * 10000.0).round() / 10000.0
    }
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
