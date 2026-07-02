# UI Language Settings Design

## Goal

Complete the "Interface Language" setting in System Settings so changing the selector has an immediate, visible effect on the desktop UI and persists across app restarts.

## Scope

The first implementation connects the language setting to the application shell and high-visibility settings workflow:

- Sidebar navigation and navigation group titles.
- Top bar labels, status bar defaults, and client connection warning copy.
- Dashboard cards, quick actions, section headings, runtime labels, and common health labels.
- System Settings page labels, hints, action buttons, connection panel, backup panel, and color labels.
- Existing enum label helpers for runtime mode, backup type, audit actions/entities, approval status, stocktake status, stocktake scope, and movement types.

Deep business forms and long table-heavy pages can continue migrating into the same i18n layer incrementally. This avoids a risky one-shot rewrite of every existing hard-coded string while making the language selector genuinely functional for acceptance.

## Architecture

Add a small frontend-only i18n module at `src/i18n.ts`.

The module owns:

- `LocaleCode` as `zh-CN | en-US`.
- A typed translation dictionary organized by stable keys.
- `createI18n(locale)`, returning `t(key, params?)`, `formatMoney(value)`, and typed label helpers.
- Simple placeholder interpolation for messages such as `{count}` and `{hours}`.

`App.tsx` keeps `appearanceSettings.locale` as the single source of truth. When it changes, the app:

- Applies visual settings.
- Persists the appearance settings to `localStorage`.
- Updates `document.documentElement.lang`.
- Recomputes `i18n` through `useMemo`.

Components receive either the `i18n` object or translated props. They do not read from localStorage and do not branch on locale directly.

## Data Flow

1. App starts and reads `aster.appearance` from `localStorage`.
2. `normalizeAppearanceSettings` validates `locale`; invalid values fall back to `zh-CN`.
3. Main app creates an i18n instance for the active locale.
4. Settings page changes `appearanceSettings.locale`.
5. React re-renders visible translated UI immediately and the effect persists the updated appearance payload.

## Error Handling

Missing translation keys should return the key itself so the UI remains usable during incremental migration. Invalid locale values are normalized to `zh-CN`.

## Testing

- Run `npm run build` to verify TypeScript and Vite.
- Run `git diff --check`.
- Manually inspect the settings language selector in the app if a dev server is running.
