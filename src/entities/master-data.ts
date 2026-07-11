export type Category = {
  id: string;
  parentId?: string | null;
  name: string;
  enabled: boolean;
  sortOrder: number;
  updatedAt: string;
};

export type Unit = { id: string; name: string; enabled: boolean; sortOrder: number; updatedAt: string };

export type Department = {
  id: string;
  code: string;
  name: string;
  manager?: string | null;
  enabled: boolean;
  sortOrder: number;
  remark?: string | null;
  updatedAt: string;
};

export type Supplier = {
  id: string;
  name: string;
  contact?: string | null;
  phone?: string | null;
  address?: string | null;
  enabled: boolean;
  remark?: string | null;
  updatedAt: string;
};

export type SupplierPurchaseRecord = {
  movementDate: string;
  documentNo?: string | null;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  remark?: string | null;
};

export type Item = {
  id: string;
  code: string;
  barcode?: string | null;
  name: string;
  categoryId?: string | null;
  categoryName?: string | null;
  spec?: string | null;
  unitId?: string | null;
  unitName?: string | null;
  defaultPrice: number;
  salePrice: number;
  supplierId?: string | null;
  supplierName?: string | null;
  warningQuantity: number;
  enabled: boolean;
  remark?: string | null;
  updatedAt: string;
};

export type BudgetRule = {
  id: string;
  departmentId: string;
  departmentName: string;
  categoryId?: string | null;
  categoryName: string;
  periodMonth: string;
  amountLimit: number;
  usedAmount: number;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
};

export type OptionRecord = { id: string; name: string; enabled: boolean };
