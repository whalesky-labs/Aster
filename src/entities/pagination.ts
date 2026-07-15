export type Page<T> = {
  items: T[];
  nextCursor?: string | null;
};

export type DataPageKey =
  | "items"
  | "inboundDocuments"
  | "outboundDocuments"
  | "adjustmentDocuments"
  | "stockBalances"
  | "stockMovements";
