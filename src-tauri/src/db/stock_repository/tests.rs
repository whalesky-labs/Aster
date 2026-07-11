use rusqlite::{params, Connection};

use super::*;
use crate::db::migrations;
use crate::db::stocktake_repository::{
    confirm_stocktake, create_stocktake, update_stocktake_counts,
};
use crate::domain::stock::{
    SubmitAdjustmentLine, SubmitAdjustmentRequest, SubmitStockDocumentLine,
    SubmitStockDocumentRequest, VoidStockDocumentRequest,
};
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, UpdateStocktakeCountsRequest,
    UpdateStocktakeLineRequest,
};
#[path = "tests/batches.rs"]
mod batches;
#[path = "tests/documents.rs"]
mod documents;
#[path = "tests/queries.rs"]
mod queries;
#[path = "tests/rules.rs"]
mod rules;
