export type ImportMessage = {
  level: string;
  sheet: string;
  row: number;
  column?: string | null;
  message: string;
};

export type ImportItemPreview = {
  name: string;
  categoryName?: string | null;
  spec?: string | null;
  unitName?: string | null;
  defaultPrice: number;
  openingQuantity: number;
  inboundQuantity: number;
  outboundQuantity: number;
  existing: boolean;
};

export type ImportMonthPreview = {
  month: string;
  rowCount: number;
  openingQuantity: number;
  inboundQuantity: number;
  outboundQuantity: number;
  outboundAmount: number;
};

export type ImportPreview = {
  sourceFile: string;
  sheetCount: number;
  rowCount: number;
  itemCount: number;
  newItemCount: number;
  existingItemCount: number;
  openingQuantity: number;
  openingAmount: number;
  inboundQuantity: number;
  inboundAmount: number;
  outboundQuantity: number;
  outboundAmount: number;
  documentCount: number;
  warnings: ImportMessage[];
  errors: ImportMessage[];
  items: ImportItemPreview[];
  months: ImportMonthPreview[];
};

export type ImportResult = {
  jobId: string;
  sourceFile: string;
  importedItems: number;
  matchedItems: number;
  documentCount: number;
  movementCount: number;
  warningCount: number;
  errorCount: number;
  reportPath?: string | null;
  sourceCopyPath?: string | null;
};
