import type { OptionRecord } from "../../entities/master-data";
import type { CurrentUser } from "../../entities/users";

export type StockDocumentLineDraft = {
  itemId: string;
  quantity: number;
  unitPrice: number;
  amount?: number | null;
  purchaseUnitPrice?: number | null;
  purchaseAmount?: number | null;
  saleUnitPrice?: number | null;
  saleAmount?: number | null;
  costUnitPrice?: number | null;
  costAmount?: number | null;
  remark: string;
};

export type StockDocumentDraft = {
  documentId?: string;
  documentType: "inbound" | "outbound";
  outboundKind?: "internal" | "guest_sale";
  businessDate: string;
  departmentId: string;
  supplierId: string;
  handler: string;
  purpose: string;
  remark: string;
  approvalRequestId: string;
  lines: StockDocumentLineDraft[];
};

export function effectiveDraftAmount(
  line: StockDocumentLineDraft,
  documentType: "inbound" | "outbound",
  outboundKind?: "internal" | "guest_sale",
) {
  if (documentType === "inbound") {
    const unitPrice = line.purchaseUnitPrice ?? line.unitPrice;
    return line.purchaseAmount && line.purchaseAmount > 0
      ? line.purchaseAmount
      : line.amount && line.amount > 0
        ? line.amount
        : line.quantity * unitPrice;
  }
  if (outboundKind === "guest_sale") {
    const unitPrice = line.saleUnitPrice ?? line.unitPrice;
    return line.saleAmount && line.saleAmount > 0
      ? line.saleAmount
      : line.amount && line.amount > 0
        ? line.amount
        : line.quantity * unitPrice;
  }
  return line.costAmount && line.costAmount > 0
    ? line.costAmount
    : line.amount && line.amount > 0
      ? line.amount
      : 0;
}

export function userDisplayName(user?: CurrentUser | null) {
  return user ? user.displayName?.trim() || user.username : "";
}

export function optionName(options: OptionRecord[], id?: string | null) {
  return options.find((item) => item.id === id)?.name ?? "-";
}

export function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}
