import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { PaginatedTable } from "./DataTable";

function PaginationHarness({ onLoad }: { onLoad: () => void }) {
  const [rows, setRows] = useState(["first", "second"]);
  const [hasMore, setHasMore] = useState(true);
  return (
    <table>
      <PaginatedTable
        colSpan={1}
        getRowKey={(row) => row}
        hasMore={hasMore}
        onLoadMore={async () => {
          onLoad();
          setRows((current) => [...current, "third"]);
          setHasMore(false);
        }}
        pageSize={1}
        rows={rows}
      >
        {(row) => <td>{row}</td>}
      </PaginatedTable>
    </table>
  );
}

describe("PaginatedTable", () => {
  it("loads the next backend page only after the loaded rows are exhausted", async () => {
    const onLoad = vi.fn();
    render(<PaginationHarness onLoad={onLoad} />);
    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    expect(screen.getByText("second")).toBeTruthy();
    expect(onLoad).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    await waitFor(() => expect(screen.getByText("third")).toBeTruthy());
    expect(onLoad).toHaveBeenCalledTimes(1);
  });
});
