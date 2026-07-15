import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import { localDate, localDateTime, localMonth } from "./localDate";

describe("local date formatting", () => {
  beforeAll(() => {
    vi.stubEnv("TZ", "Asia/Shanghai");
  });

  afterAll(() => {
    vi.unstubAllEnvs();
  });

  it("keeps the local month during the first eight hours of a Shanghai month", () => {
    const instant = new Date("2026-06-30T16:30:00.000Z");
    expect(localMonth(instant)).toBe("2026-07");
    expect(localDate(instant)).toBe("2026-07-01");
  });

  it("pads single-digit months and days", () => {
    expect(localDate(new Date(2026, 0, 3, 12))).toBe("2026-01-03");
  });

  it("formats a datetime-local value without converting it to UTC", () => {
    expect(localDateTime(new Date(2026, 0, 3, 4, 5, 6))).toBe(
      "2026-01-03T04:05:06",
    );
  });
});
