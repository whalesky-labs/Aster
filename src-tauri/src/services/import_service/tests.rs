use calamine::CellErrorType;
use rusqlite::Connection;
use rust_xlsxwriter::Workbook;
use tempfile::tempdir;
use zip::ZipArchive;

use super::*;
use crate::app::paths::AppPaths;
use crate::db::connection::Db;
use crate::db::migrations;
use crate::db::repository;

include!("tests/workbook.rs");
include!("tests/execution.rs");
