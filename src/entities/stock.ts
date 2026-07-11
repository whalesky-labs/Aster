export type StockDocument = {
  id: string;
  documentNo: string;
  documentType: "inbound" | "outbound" | "adjustment" | "stocktake";
  outboundKind?: "internal" | "guest_sale" | null;
  businessDate: string;
  departmentName?: string | null;
  supplierName?: string | null;
  handler?: string | null;
  purpose?: string | null;
  approvalRequestId?: string | null;
  status: string;
  totalQuantity: number;
  totalAmount: number;
  totalPurchaseAmount: number;
  totalSaleAmount: number;
  totalCostAmount: number;
  totalGrossProfit: number;
  itemSummary?: string | null;
  createdAt: string;
};

export type StockDocumentDetail = {
  document: StockDocument;
  lines: StockDocumentLine[];
  batchLines: StockDocumentBatchLine[];
};

export type StockDocumentLine = {
  id: string;
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  purchaseUnitPrice?: number | null;
  purchaseAmount?: number | null;
  saleUnitPrice?: number | null;
  saleAmount?: number | null;
  costUnitPrice?: number | null;
  costAmount?: number | null;
  grossProfit?: number | null;
  remark?: string | null;
};

export type StockDocumentBatchLine = {
  id: string;
  itemId: string;
  itemCode: string;
  itemName: string;
  batchId: string;
  batchNo: string;
  inboundDate: string;
  supplierName?: string | null;
  direction: "in" | "out";
  quantity: number;
  unitPrice: number;
  amount: number;
  movementType: string;
  createdAt: string;
};

export type StockDocumentQuery = {
  documentType: "inbound" | "outbound" | "adjustment" | "stocktake";
  outboundKind?: "internal" | "guest_sale" | null;
  month?: string | null;
  departmentId?: string | null;
  supplierId?: string | null;
  itemId?: string | null;
  handler?: string | null;
  search?: string | null;
};

export type StockBalanceQuery = {
  search?: string | null;
  categoryId?: string | null;
  itemId?: string | null;
  stockStatus?: "normal" | "low" | "negative" | null;
};

export type StockMovementQuery = {
  search?: string | null;
  itemId?: string | null;
  departmentId?: string | null;
  direction?: "in" | "out" | null;
  movementType?: string | null;
};

export type StockBalanceRow = {
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
  stockStatus: "normal" | "low" | "negative";
};

export type StockBatchRow = {
  id: string;
  itemId: string;
  itemCode: string;
  itemName: string;
  batchNo: string;
  inboundDate: string;
  supplierName?: string | null;
  originalQuantity: number;
  remainingQuantity: number;
  unitPrice: number;
  originalAmount: number;
  remainingAmount: number;
  status: string;
  sourceDocumentNo?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type StockMovementRow = {
  id: string;
  movementDate: string;
  itemCode: string;
  itemName: string;
  direction: "in" | "out";
  quantity: number;
  unitPrice: number;
  amount: number;
  documentNo?: string | null;
  departmentName?: string | null;
  supplierName?: string | null;
  movementType: string;
  operator?: string | null;
  remark?: string | null;
  createdAt: string;
};

export type StocktakeDocument = {
  id: string;
  documentId: string;
  documentNo: string;
  businessDate: string;
  scopeType: "all" | "category" | "custom";
  status: string;
  handler?: string | null;
  remark?: string | null;
  lineCount: number;
  countedCount: number;
  differenceCount: number;
  gainAmount: number;
  lossAmount: number;
  createdAt: string;
  confirmedAt?: string | null;
};

export type StocktakeLine = {
  id: string;
  stocktakeId: string;
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  bookQuantity: number;
  countedQuantity?: number | null;
  differenceQuantity: number;
  averagePrice: number;
  differenceAmount: number;
  remark?: string | null;
};

export type StocktakeDetail = {
  document: StocktakeDocument;
  lines: StocktakeLine[];
};
