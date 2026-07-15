import { describe, expect, it } from "vitest";

import { refreshTargetForEditor } from "./refreshTargets";

describe("targeted refresh routing", () => {
  it("refreshes only the domain affected by an editor save", () => {
    expect(refreshTargetForEditor("item")).toBe("master");
    expect(refreshTargetForEditor("stockDocument")).toBe("stock");
    expect(refreshTargetForEditor("connectionWizard")).toBe("connection");
    expect(refreshTargetForEditor("businessSettings")).toBe("admin");
    expect(refreshTargetForEditor("stockDocumentDetail")).toBe("none");
  });
});
