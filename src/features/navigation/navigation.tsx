import type { ReactNode } from "react";
import type { NavKey } from "../../entities/navigation";

export type NavItem = { key: NavKey; labelKey: string };
const navIconProps = {
  "aria-hidden": true,
  className: "nav-item-icon",
  fill: "none",
  focusable: "false",
  stroke: "currentColor",
  strokeLinecap: "round",
  strokeLinejoin: "round",
  strokeWidth: 1.8,
  viewBox: "0 0 24 24",
} as const;

const navIconContent: Record<NavKey, ReactNode> = {
  dashboard: (
    <>
      <path d="M4 11.4 12 4l8 7.4" />
      <path d="M6.5 10.5V20h11v-9.5" />
      <path d="M10 20v-5h4v5" />
    </>
  ),
  items: (
    <>
      <path d="M6 7.5 12 4l6 3.5v7L12 18l-6-3.5z" />
      <path d="m6.3 8 5.7 3.3L17.7 8" />
      <path d="M12 11.3V18" />
    </>
  ),
  inbound: (
    <>
      <path d="M5 19h14V9H5z" />
      <path d="M8 9V6h8v3" />
      <path d="M12 5v8" />
      <path d="m9 10 3 3 3-3" />
    </>
  ),
  outbound: (
    <>
      <path d="M5 19h14V9H5z" />
      <path d="M8 9V6h8v3" />
      <path d="M12 14V5" />
      <path d="m9 8 3-3 3 3" />
    </>
  ),
  stock: (
    <>
      <path d="M5 8h14v11H5z" />
      <path d="M7 5h10v3H7z" />
      <path d="M9 12h6" />
      <path d="M9 16h4" />
    </>
  ),
  movements: (
    <>
      <path d="M4 7h11" />
      <path d="m12 4 3 3-3 3" />
      <path d="M20 17H9" />
      <path d="m12 14-3 3 3 3" />
    </>
  ),
  stocktake: (
    <>
      <path d="M8 4h8l2 3v13H6V7z" />
      <path d="M8 7h8" />
      <path d="m9 13 2 2 4-5" />
    </>
  ),
  adjustments: (
    <>
      <path d="M5 7h14" />
      <path d="M8 7v10" />
      <path d="M16 7v10" />
      <path d="M6 17h6" />
      <path d="M14 17h4" />
      <path d="M10 13h8" />
    </>
  ),
  reports: (
    <>
      <path d="M5 20V5" />
      <path d="M5 20h14" />
      <path d="M8.5 16v-4" />
      <path d="M12 16V8" />
      <path d="M15.5 16v-6" />
    </>
  ),
  import: (
    <>
      <path d="M6 4h8l4 4v12H6z" />
      <path d="M14 4v4h4" />
      <path d="M12 10v6" />
      <path d="m9 13 3 3 3-3" />
    </>
  ),
  departments: (
    <>
      <path d="M4 20h16" />
      <path d="M6 20V8h6v12" />
      <path d="M12 20V5h6v15" />
      <path d="M8 11h2" />
      <path d="M14 9h2" />
      <path d="M14 13h2" />
    </>
  ),
  categories: (
    <>
      <path d="M5 6h5l2 2h7v10H5z" />
      <path d="M8 13h8" />
      <path d="M8 16h5" />
    </>
  ),
  units: (
    <>
      <path d="M6 18h12" />
      <path d="M8 6v12" />
      <path d="M16 6v12" />
      <path d="M8 8h8" />
      <path d="M8 14h8" />
    </>
  ),
  suppliers: (
    <>
      <path d="M4 17h2" />
      <path d="M18 17h2" />
      <path d="M6 17h12" />
      <path d="M7 13V7h7v10" />
      <path d="M14 10h3l2 3v4" />
      <path d="M8 17a2 2 0 1 0 4 0" />
      <path d="M16 17a2 2 0 1 0 4 0" />
    </>
  ),
  budgets: (
    <>
      <path d="M6 5h12v14H6z" />
      <path d="M9 9h6" />
      <path d="M9 13h2" />
      <path d="M13 13h2" />
      <path d="M9 16h6" />
    </>
  ),
  approvals: (
    <>
      <path d="M7 4h10v16H7z" />
      <path d="M9.5 8h5" />
      <path d="m9.5 14 2 2 3.5-4" />
    </>
  ),
  backups: (
    <>
      <path d="M5 7h14v12H5z" />
      <path d="M8 7V5h8v2" />
      <path d="M8 12h8" />
      <path d="M8 15h5" />
    </>
  ),
  logs: (
    <>
      <path d="M6 5h12v14H6z" />
      <path d="M9 9h6" />
      <path d="M9 12h6" />
      <path d="M9 15h4" />
      <path d="M6 5 4.5 6.5" />
      <path d="M18 5l1.5 1.5" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 4v2" />
      <path d="M12 18v2" />
      <path d="m5.6 5.6 1.4 1.4" />
      <path d="m17 17 1.4 1.4" />
      <path d="M4 12h2" />
      <path d="M18 12h2" />
      <path d="m5.6 18.4 1.4-1.4" />
      <path d="m17 7 1.4-1.4" />
    </>
  ),
  users: (
    <>
      <circle cx="9" cy="8" r="3" />
      <path d="M4.5 19a4.5 4.5 0 0 1 9 0" />
      <path d="M16 10a2.5 2.5 0 0 1 0 5" />
      <path d="M18 19a4 4 0 0 0-3-3.8" />
    </>
  ),
};

export function NavIcon({ name }: { name: NavKey }) {
  return <svg {...navIconProps}>{navIconContent[name]}</svg>;
}

export function GitHubIcon() {
  return (
    <svg
      aria-hidden="true"
      className="topbar-icon"
      focusable="false"
      viewBox="0 0 24 24"
    >
      <path
        d="M12 2.25a9.75 9.75 0 0 0-3.08 19c.49.09.67-.21.67-.47v-1.67c-2.72.59-3.3-1.18-3.3-1.18-.44-1.13-1.08-1.43-1.08-1.43-.88-.6.07-.59.07-.59.98.07 1.49 1 1.49 1 .87 1.48 2.27 1.05 2.82.8.09-.62.34-1.05.61-1.29-2.17-.25-4.45-1.09-4.45-4.83 0-1.07.38-1.94 1-2.62-.1-.25-.43-1.24.1-2.58 0 0 .82-.26 2.68 1a9.2 9.2 0 0 1 4.88 0c1.86-1.26 2.68-1 2.68-1 .53 1.34.2 2.33.1 2.58.62.68 1 1.55 1 2.62 0 3.75-2.28 4.58-4.46 4.82.35.3.66.9.66 1.82v2.7c0 .26.18.56.68.46A9.75 9.75 0 0 0 12 2.25Z"
        fill="currentColor"
      />
    </svg>
  );
}

export const navItems: NavItem[] = [
  { key: "dashboard", labelKey: "nav.dashboard" },
  { key: "items", labelKey: "nav.items" },
  { key: "inbound", labelKey: "nav.inbound" },
  { key: "outbound", labelKey: "nav.outbound" },
  { key: "stock", labelKey: "nav.stock" },
  { key: "movements", labelKey: "nav.movements" },
  { key: "stocktake", labelKey: "nav.stocktake" },
  { key: "adjustments", labelKey: "nav.adjustments" },
  { key: "reports", labelKey: "nav.reports" },
  { key: "import", labelKey: "nav.import" },
  { key: "departments", labelKey: "nav.departments" },
  { key: "categories", labelKey: "nav.categories" },
  { key: "units", labelKey: "nav.units" },
  { key: "suppliers", labelKey: "nav.suppliers" },
  { key: "budgets", labelKey: "nav.budgets" },
  { key: "approvals", labelKey: "nav.approvals" },
  { key: "backups", labelKey: "nav.backups" },
  { key: "logs", labelKey: "nav.logs" },
  { key: "settings", labelKey: "nav.settings" },
  { key: "users", labelKey: "nav.users" },
];

export const navGroups: { titleKey: string; keys: NavKey[] }[] = [
  { titleKey: "navGroup.overview", keys: ["dashboard"] },
  {
    titleKey: "navGroup.inventory",
    keys: [
      "items",
      "inbound",
      "outbound",
      "stocktake",
      "adjustments",
    ],
  },
  {
    titleKey: "navGroup.reports",
    keys: ["stock", "movements", "reports"],
  },
  {
    titleKey: "navGroup.masterData",
    keys: ["departments", "categories", "units", "suppliers"],
  },
  {
    titleKey: "navGroup.management",
    keys: ["import", "budgets", "approvals", "users"],
  },
  {
    titleKey: "navGroup.logs",
    keys: ["backups", "logs"],
  },
];

export const workstreams = [
  {
    titleKey: "workstream.inventory.title",
    bodyKey: "workstream.inventory.body",
  },
  {
    titleKey: "workstream.host.title",
    bodyKey: "workstream.host.body",
  },
  {
    titleKey: "workstream.recovery.title",
    bodyKey: "workstream.recovery.body",
  },
  {
    titleKey: "workstream.delivery.title",
    bodyKey: "workstream.delivery.body",
  },
];
