PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS categories (
  id TEXT PRIMARY KEY,
  parent_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(parent_id, name)
);

CREATE TABLE IF NOT EXISTS units (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS departments (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  manager TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS suppliers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  contact TEXT,
  phone TEXT,
  address TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS master_items (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  barcode TEXT UNIQUE,
  name TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
  spec TEXT,
  unit_id TEXT REFERENCES units(id) ON DELETE SET NULL,
  default_price REAL NOT NULL DEFAULT 0,
  sale_price REAL NOT NULL DEFAULT 0,
  supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
  warning_quantity REAL NOT NULL DEFAULT 0,
  enabled INTEGER NOT NULL DEFAULT 1,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  email TEXT,
  password_hash TEXT,
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS password_reset_codes (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  code_hash TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_password_reset_codes_user_id ON password_reset_codes(user_id);

CREATE TABLE IF NOT EXISTS roles (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_roles (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  PRIMARY KEY(user_id, role_id)
);

CREATE TABLE IF NOT EXISTS stock_documents (
  id TEXT PRIMARY KEY,
  document_no TEXT NOT NULL UNIQUE,
  document_type TEXT NOT NULL CHECK(document_type IN ('inbound', 'outbound', 'stocktake', 'adjustment')),
  outbound_kind TEXT CHECK(outbound_kind IS NULL OR outbound_kind IN ('internal', 'guest_sale')),
  business_date TEXT NOT NULL,
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  department_name TEXT,
  supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
  supplier_name TEXT,
  handler TEXT,
  purpose TEXT,
  approval_request_id TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'confirmed', 'voided')),
  remark TEXT,
  created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  confirmed_at TEXT,
  voided_at TEXT
);

CREATE TABLE IF NOT EXISTS stock_document_lines (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES stock_documents(id) ON DELETE CASCADE,
  item_id TEXT NOT NULL REFERENCES master_items(id) ON DELETE RESTRICT,
  quantity REAL NOT NULL,
  unit_price REAL NOT NULL DEFAULT 0,
  amount REAL NOT NULL DEFAULT 0,
  purchase_unit_price REAL,
  purchase_amount REAL,
  sale_unit_price REAL,
  sale_amount REAL,
  cost_unit_price REAL,
  cost_amount REAL,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stock_movements (
  id TEXT PRIMARY KEY,
  movement_date TEXT NOT NULL,
  item_id TEXT NOT NULL REFERENCES master_items(id) ON DELETE RESTRICT,
  batch_id TEXT,
  direction TEXT NOT NULL CHECK(direction IN ('in', 'out')),
  quantity REAL NOT NULL,
  unit_price REAL NOT NULL DEFAULT 0,
  amount REAL NOT NULL DEFAULT 0,
  document_id TEXT REFERENCES stock_documents(id) ON DELETE SET NULL,
  document_line_id TEXT REFERENCES stock_document_lines(id) ON DELETE SET NULL,
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  department_name TEXT,
  supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
  supplier_name TEXT,
  movement_type TEXT NOT NULL CHECK(movement_type IN ('opening', 'inbound', 'outbound', 'stocktake_gain', 'stocktake_loss', 'adjustment', 'reversal')),
  operator TEXT,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stock_batches (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES master_items(id) ON DELETE RESTRICT,
  source_document_id TEXT REFERENCES stock_documents(id) ON DELETE SET NULL,
  source_document_line_id TEXT REFERENCES stock_document_lines(id) ON DELETE SET NULL,
  batch_no TEXT NOT NULL UNIQUE,
  inbound_date TEXT NOT NULL,
  supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
  supplier_name TEXT,
  original_quantity REAL NOT NULL,
  remaining_quantity REAL NOT NULL,
  unit_price REAL NOT NULL DEFAULT 0,
  original_amount REAL NOT NULL DEFAULT 0,
  remaining_amount REAL NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'available' CHECK(status IN ('available', 'depleted', 'voided', 'adjustment')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stock_batch_movements (
  id TEXT PRIMARY KEY,
  batch_id TEXT NOT NULL REFERENCES stock_batches(id) ON DELETE RESTRICT,
  stock_movement_id TEXT REFERENCES stock_movements(id) ON DELETE SET NULL,
  document_id TEXT REFERENCES stock_documents(id) ON DELETE SET NULL,
  document_line_id TEXT REFERENCES stock_document_lines(id) ON DELETE SET NULL,
  direction TEXT NOT NULL CHECK(direction IN ('in', 'out')),
  quantity REAL NOT NULL,
  unit_price REAL NOT NULL DEFAULT 0,
  amount REAL NOT NULL DEFAULT 0,
  movement_type TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stock_balances (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL UNIQUE REFERENCES master_items(id) ON DELETE CASCADE,
  quantity REAL NOT NULL DEFAULT 0,
  amount REAL NOT NULL DEFAULT 0,
  average_price REAL NOT NULL DEFAULT 0,
  last_inbound_price REAL NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stocktake_documents (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL UNIQUE REFERENCES stock_documents(id) ON DELETE CASCADE,
  scope_type TEXT NOT NULL DEFAULT 'all' CHECK(scope_type IN ('all', 'category', 'custom')),
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'counting', 'confirmed', 'voided')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS stocktake_lines (
  id TEXT PRIMARY KEY,
  stocktake_id TEXT NOT NULL REFERENCES stocktake_documents(id) ON DELETE CASCADE,
  item_id TEXT NOT NULL REFERENCES master_items(id) ON DELETE RESTRICT,
  book_quantity REAL NOT NULL DEFAULT 0,
  counted_quantity REAL,
  difference_quantity REAL NOT NULL DEFAULT 0,
  remark TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(stocktake_id, item_id)
);

CREATE TABLE IF NOT EXISTS approval_requests (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected', 'cancelled')),
  requested_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  decided_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  reason TEXT,
  decision_note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  decided_at TEXT
);

CREATE TABLE IF NOT EXISTS budget_rules (
  id TEXT PRIMARY KEY,
  department_id TEXT REFERENCES departments(id) ON DELETE CASCADE,
  category_id TEXT REFERENCES categories(id) ON DELETE CASCADE,
  period_month TEXT NOT NULL,
  amount_limit REAL NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS import_jobs (
  id TEXT PRIMARY KEY,
  source_file TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'previewed', 'imported', 'failed')),
  total_rows INTEGER NOT NULL DEFAULT 0,
  success_rows INTEGER NOT NULL DEFAULT 0,
  warning_rows INTEGER NOT NULL DEFAULT 0,
  error_rows INTEGER NOT NULL DEFAULT 0,
  report_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS backup_jobs (
  id TEXT PRIMARY KEY,
  backup_file TEXT NOT NULL,
  backup_type TEXT NOT NULL CHECK(backup_type IN ('auto_startup', 'auto_interval', 'manual', 'before_import', 'before_restore', 'before_migration')),
  app_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  host_name TEXT,
  os TEXT,
  database_size INTEGER NOT NULL DEFAULT 0,
  sha256 TEXT,
  status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'success', 'failed')),
  error_message TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT,
  summary TEXT NOT NULL,
  operator TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS client_connections (
  id TEXT PRIMARY KEY,
  client_name TEXT NOT NULL,
  client_device_id TEXT NOT NULL,
  token_hash TEXT NOT NULL DEFAULT '',
  client_ip TEXT,
  app_version TEXT,
  status TEXT NOT NULL DEFAULT 'paired',
  last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_client_connections_device_id ON client_connections(client_device_id);

CREATE INDEX IF NOT EXISTS idx_master_items_code ON master_items(code);
CREATE INDEX IF NOT EXISTS idx_master_items_barcode ON master_items(barcode);
CREATE INDEX IF NOT EXISTS idx_master_items_name ON master_items(name);
CREATE INDEX IF NOT EXISTS idx_stock_documents_type_status ON stock_documents(document_type, status);
CREATE INDEX IF NOT EXISTS idx_stock_documents_business_date ON stock_documents(business_date);
CREATE INDEX IF NOT EXISTS idx_stock_movements_date ON stock_movements(movement_date);
CREATE INDEX IF NOT EXISTS idx_stock_movements_item ON stock_movements(item_id);
CREATE INDEX IF NOT EXISTS idx_stock_movements_department ON stock_movements(department_id);
CREATE INDEX IF NOT EXISTS idx_stock_movements_type ON stock_movements(movement_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_rules_department_month
  ON budget_rules(department_id, period_month)
  WHERE category_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_rules_department_category_month
  ON budget_rules(department_id, category_id, period_month)
  WHERE category_id IS NOT NULL;

INSERT OR IGNORE INTO roles (id, code, name) VALUES
  ('role-admin', 'admin', '管理员'),
  ('role-warehouse', 'warehouse', '仓库员'),
  ('role-department-viewer', 'department_viewer', '部门查看员'),
  ('role-readonly', 'readonly', '只读用户');

INSERT OR IGNORE INTO departments (id, code, name, sort_order) VALUES
  ('dept-admin-office', 'D001', '行政办', 1),
  ('dept-restaurant', 'D002', '餐饮', 2),
  ('dept-hot-spring-front', 'D003', '温泉+前台', 3),
  ('dept-housekeeping', 'D004', '客房', 4),
  ('dept-engineering', 'D005', '工程', 5),
  ('dept-security', 'D006', '安保', 6),
  ('dept-women-home', 'D007', '妇女之家', 7),
  ('dept-transfer', 'D008', '调物品', 8);

INSERT OR IGNORE INTO units (id, name, sort_order) VALUES
  ('unit-piece', '件', 1),
  ('unit-box', '盒', 2),
  ('unit-pack', '包', 3),
  ('unit-bottle', '瓶', 4),
  ('unit-set', '套', 5);

INSERT OR IGNORE INTO app_settings (key, value) VALUES
  ('hotel_name', 'Aster Hotel'),
  ('runtime_mode', 'standalone'),
  ('allow_negative_stock', 'false'),
  ('quantity_decimals', '2'),
  ('amount_decimals', '2'),
  ('host_port', '17871'),
  ('auto_backup_enabled', 'true'),
  ('schema_version', '1');

INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (1, 'initial_schema');
