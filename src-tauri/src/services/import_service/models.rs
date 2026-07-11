#[derive(Debug, Clone)]
struct ParsedWorkbook {
    items: Vec<ParsedItem>,
    inbound_rows: Vec<ParsedInboundLine>,
    outbound_rows: Vec<ParsedOutboundLine>,
    warnings: Vec<ImportMessage>,
    errors: Vec<ImportMessage>,
    sheet_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedItem {
    key: String,
    sheet_name: String,
    row_number: usize,
    code: Option<String>,
    name: String,
    category_name: Option<String>,
    spec: Option<String>,
    unit_name: Option<String>,
    default_price: f64,
    sale_price: f64,
    warning_quantity: f64,
    remark: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedInboundLine {
    sheet_name: String,
    row_number: usize,
    business_date: String,
    supplier_name: Option<String>,
    item_key: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    handler: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedOutboundLine {
    sheet_name: String,
    row_number: usize,
    business_date: String,
    outbound_kind: String,
    department_name: String,
    item_key: String,
    quantity: f64,
    sale_unit_price: f64,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InboundGroupKey {
    business_date: String,
    supplier_name: Option<String>,
    handler: Option<String>,
    remark: Option<String>,
}

struct InboundGroup<'a> {
    business_date: String,
    supplier_name: Option<String>,
    handler: Option<String>,
    remark: Option<String>,
    rows: Vec<&'a ParsedInboundLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OutboundGroupKey {
    business_date: String,
    outbound_kind: String,
    department_name: String,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
}

struct OutboundGroup<'a> {
    business_date: String,
    outbound_kind: String,
    department_name: String,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
    rows: Vec<&'a ParsedOutboundLine>,
}

struct ImportItemAccumulator {
    name: String,
    category_name: Option<String>,
    spec: Option<String>,
    unit_name: Option<String>,
    default_price: f64,
    opening_quantity: f64,
    inbound_quantity: f64,
    outbound_quantity: f64,
    existing: bool,
}

#[derive(Default)]
struct ImportMonthAccumulator {
    row_count: usize,
    inbound_quantity: f64,
    outbound_quantity: f64,
    outbound_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportReportFile {
    job_id: String,
    source_file: String,
    source_copy_path: Option<String>,
    mode: String,
    generated_at: String,
    sheet_count: usize,
    row_count: usize,
    item_count: usize,
    new_item_count: usize,
    existing_item_count: usize,
    imported_items: usize,
    matched_items: usize,
    document_count: usize,
    movement_count: usize,
    warning_count: usize,
    error_count: usize,
    months: Vec<ImportMonthPreview>,
    warnings: Vec<ImportMessage>,
    errors: Vec<ImportMessage>,
    items: Vec<ImportItemPreview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMode {
    Full,
    ItemsOnly,
}

impl ImportMode {
    fn from_request(value: Option<&str>) -> AppResult<Self> {
        match value.unwrap_or("full") {
            "full" => Ok(Self::Full),
            "itemsOnly" | "items_only" => Ok(Self::ItemsOnly),
            other => Err(AppError::Validation(format!("不支持的导入模式：{other}"))),
        }
    }

    fn import_movements(self) -> bool {
        self == Self::Full
    }

    fn label(self) -> &'static str {
        match self {
            Self::Full => "完整导入",
            Self::ItemsOnly => "只导入物品档案",
        }
    }
}

struct SheetHeader {
    row_index: usize,
    columns: HashMap<String, usize>,
}

impl SheetHeader {
    fn from_range(
        range: &calamine::Range<Data>,
        sheet_name: &str,
        required: &[&str],
    ) -> AppResult<Self> {
        for (row_index, row) in range.rows().take(10).enumerate() {
            let mut columns = HashMap::new();
            for (index, cell) in row.iter().enumerate() {
                let text = data_to_text(cell);
                if text.trim().is_empty() {
                    continue;
                }
                columns.insert(normalized_header(&text), index);
            }
            if required
                .iter()
                .all(|label| columns.contains_key(&normalized_header(label)))
            {
                return Ok(Self { row_index, columns });
            }
        }
        Err(AppError::Validation(format!(
            "{sheet_name} 表头不匹配，缺少必填列：{}",
            required.join("、")
        )))
    }

    fn text(&self, row: &[Data], label: &str) -> String {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .map(data_to_text)
            .unwrap_or_default()
    }

    fn optional_text(&self, row: &[Data], label: &str) -> Option<String> {
        let value = self.text(row, label);
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn optional_number(&self, row: &[Data], label: &str) -> Option<f64> {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .and_then(data_to_number)
    }

    fn datetime(&self, row: &[Data], label: &str) -> Result<Option<String>, String> {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .map(data_to_datetime)
            .unwrap_or(Ok(None))
    }
}

#[allow(dead_code)]
fn _assert_preview_serializable<T: Serialize>(_value: &T) {}
