import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ItemsPage } from "./master-data/ItemsPage";
import { StockBalancePage } from "./stock/StockBalancePage";

function renderItemsPage(onSearch: (search: string, supplierId: string) => Promise<void>) {
  return render(
    <ItemsPage
      canImportItems={false}
      canWrite={false}
      categories={[]}
      formatMoney={(value) => String(value)}
      hasMore={false}
      itemSearch=""
      itemSupplierId=""
      items={[]}
      onCreate={() => undefined}
      onEdit={() => undefined}
      onExportItems={async () => undefined}
      onImportItems={async () => undefined}
      onLoadMore={async () => undefined}
      onSearch={onSearch}
      onToggle={async () => undefined}
      suppliers={[{ id: "supplier-a", name: "供应商 A", enabled: true }]}
      units={[]}
    />,
  );
}

describe("inventory feature controls", () => {
  it("submits and clears the item supplier dropdown with the keyword", async () => {
    const onSearch = vi.fn(async () => undefined);
    renderItemsPage(onSearch);

    fireEvent.change(screen.getByLabelText("关键字"), {
      target: { value: "客房" },
    });
    fireEvent.change(screen.getByLabelText("供应商"), {
      target: { value: "supplier-a" },
    });
    fireEvent.click(screen.getByRole("button", { name: "筛选" }));
    await waitFor(() =>
      expect(onSearch).toHaveBeenLastCalledWith("客房", "supplier-a"),
    );

    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    await waitFor(() => expect(onSearch).toHaveBeenLastCalledWith("", ""));
    expect((screen.getByLabelText("供应商") as HTMLSelectElement).value).toBe("");
  });

  it("shows the full inventory export only to administrators", async () => {
    const onExport = vi.fn(async () => undefined);
    const baseProps = {
      balances: [],
      categories: [],
      hasMore: false,
      items: [],
      onExport,
      onLoadMore: async () => undefined,
      onQueryChange: async () => undefined,
      onViewBatches: () => undefined,
      onViewMovements: async () => undefined,
      query: {},
    };
    const { rerender } = render(
      <StockBalancePage {...baseProps} canExport={false} />,
    );
    expect(
      screen.queryByRole("button", { name: "导出全部库存" }),
    ).toBeNull();

    rerender(<StockBalancePage {...baseProps} canExport />);
    fireEvent.click(screen.getByRole("button", { name: "导出全部库存" }));
    await waitFor(() => expect(onExport).toHaveBeenCalledTimes(1));
  });
});
