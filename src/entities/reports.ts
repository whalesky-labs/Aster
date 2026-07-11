export type MonthlyInventoryRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  inboundQuantity: number;
  inboundAmount: number;
  outboundQuantity: number;
  outboundAmount: number;
  endingQuantity: number;
  endingAmount: number;
};

export type DepartmentIssueSummaryRow = {
  departmentId: string;
  departmentName: string;
  quantity: number;
  amount: number;
};

export type DepartmentIssueDetailRow = {
  movementDate: string;
  departmentName: string;
  outboundKind?: "internal" | "guest_sale" | null;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  saleUnitPrice?: number | null;
  saleAmount?: number | null;
  costUnitPrice: number;
  costAmount: number;
  grossProfit?: number | null;
  grossMargin?: number | null;
  documentNo?: string | null;
  purpose?: string | null;
  remark?: string | null;
};

export type SalesProfitRow = {
  movementDate: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  saleUnitPrice: number;
  saleAmount: number;
  costUnitPrice: number;
  costAmount: number;
  grossProfit: number;
  grossMargin?: number | null;
  negativeProfit: boolean;
  documentNo?: string | null;
  purpose?: string | null;
  remark?: string | null;
};

export type CategoryConsumptionRow = {
  categoryId?: string | null;
  categoryName: string;
  quantity: number;
  amount: number;
};

export type ItemConsumptionRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  amount: number;
};

export type InboundDetailRow = {
  movementDate: string;
  supplierName: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  documentNo?: string | null;
  remark?: string | null;
};

export type StockWarningRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  warningQuantity: number;
  shortageQuantity: number;
  amount: number;
};

export type StockBalanceReportRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  amount: number;
  averagePrice: number;
  lastInboundPrice: number;
  warningQuantity: number;
  stockStatus: string;
};

export type StocktakeDifferenceReportRow = {
  businessDate: string;
  documentNo: string;
  scopeType: string;
  status: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  bookQuantity: number;
  countedQuantity: number;
  differenceQuantity: number;
  averagePrice: number;
  differenceAmount: number;
  remark?: string | null;
};

export type ReportBundle = {
  month: string;
  monthlyInventory: MonthlyInventoryRow[];
  departmentSummary: DepartmentIssueSummaryRow[];
  departmentDetails: DepartmentIssueDetailRow[];
  categoryConsumption: CategoryConsumptionRow[];
  itemConsumptionRanking: ItemConsumptionRow[];
  inboundDetails: InboundDetailRow[];
  outboundDetails: DepartmentIssueDetailRow[];
  salesProfit: SalesProfitRow[];
  stockBalances: StockBalanceReportRow[];
  stockWarnings: StockWarningRow[];
  stocktakeDifferences: StocktakeDifferenceReportRow[];
};

export type ReportQuery = {
  month: string;
  startDate?: string | null;
  endDate?: string | null;
  departmentId?: string | null;
  categoryId?: string | null;
  itemId?: string | null;
  supplierId?: string | null;
};
