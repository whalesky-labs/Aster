import { type CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { Item } from "../../entities/master-data";
import { normalizeSearchText } from "../lib/display";

function itemSearchText(item: Item) {
  return normalizeSearchText(
    [item.code, item.barcode, item.name, item.spec, item.unitName, item.categoryName, item.supplierName]
      .filter(Boolean)
      .join(" "),
  );
}

function itemDisplayName(item: Item) {
  return [item.code, item.name].filter(Boolean).join(" · ");
}

export function ItemSearchSelect({
  allowEmpty = false,
  disabled,
  emptyLabel = "全部",
  items,
  onChange,
  onCommit,
  placeholder = "搜索编码、条码或物品名称",
  value,
}: {
  allowEmpty?: boolean;
  disabled: boolean;
  emptyLabel?: string;
  items: Item[];
  onChange: (itemId: string) => void;
  onCommit?: (itemId: string) => void;
  placeholder?: string;
  value: string;
}) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});
  const selectedItem = items.find((item) => item.id === value);
  const selectedLabel = selectedItem ? itemDisplayName(selectedItem) : "";
  const [query, setQuery] = useState(selectedLabel);
  const [open, setOpen] = useState(false);
  const normalizedQuery = normalizeSearchText(query);
  const options = useMemo(() => {
    const scored = items
      .map((item, index) => {
        const haystack = itemSearchText(item);
        const code = normalizeSearchText(item.code);
        const barcode = normalizeSearchText(item.barcode);
        const name = normalizeSearchText(item.name);
        const spec = normalizeSearchText(item.spec);
        if (!normalizedQuery) return { item, index, score: index };
        if (code.startsWith(normalizedQuery)) return { item, index, score: 0 };
        if (barcode.startsWith(normalizedQuery)) {
          return { item, index, score: 1 };
        }
        if (name.startsWith(normalizedQuery)) return { item, index, score: 2 };
        if (haystack.includes(normalizedQuery)) {
          return { item, index, score: spec.includes(normalizedQuery) ? 4 : 3 };
        }
        return null;
      })
      .filter((entry): entry is { item: Item; index: number; score: number } =>
        Boolean(entry),
      )
      .sort((left, right) => left.score - right.score || left.index - right.index)
      .slice(0, 30);
    return scored.map((entry) => entry.item);
  }, [items, normalizedQuery]);

  useEffect(() => {
    if (!open) {
      setQuery(selectedLabel);
    }
  }, [open, selectedLabel]);

  useEffect(() => {
    if (!open) return;
    function updateMenuPosition() {
      const rect = inputRef.current?.getBoundingClientRect();
      if (!rect) return;
      const viewportWidth = window.innerWidth;
      const horizontalPadding = 8;
      const width = Math.min(
        Math.max(rect.width, 260),
        viewportWidth - horizontalPadding * 2,
      );
      const left = Math.min(
        Math.max(horizontalPadding, rect.left),
        Math.max(horizontalPadding, viewportWidth - width - horizontalPadding),
      );
      setMenuStyle({
        left,
        top: rect.bottom + 4,
        width,
      });
    }
    updateMenuPosition();
    window.addEventListener("resize", updateMenuPosition);
    window.addEventListener("scroll", updateMenuPosition, true);
    return () => {
      window.removeEventListener("resize", updateMenuPosition);
      window.removeEventListener("scroll", updateMenuPosition, true);
    };
  }, [open]);

  function selectItem(item: Item) {
    onChange(item.id);
    onCommit?.(item.id);
    setQuery(itemDisplayName(item));
    setOpen(false);
  }

  function clearSelection() {
    onChange("");
    onCommit?.("");
    setQuery("");
    setOpen(true);
  }

  function commitCurrentQuery() {
    if (disabled) return;
    if (value) {
      onCommit?.(value);
      setOpen(false);
      return;
    }
    const firstOption = options[0];
    if (firstOption) {
      selectItem(firstOption);
    } else if (allowEmpty) {
      onCommit?.("");
      setOpen(false);
    }
  }

  const menu =
    open && !disabled ? (
      <div className="item-search-menu" style={menuStyle}>
        {allowEmpty ? (
          <button
            className={!value ? "selected" : ""}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              onChange("");
              onCommit?.("");
              setQuery("");
              setOpen(false);
            }}
            type="button"
          >
            <strong>{emptyLabel}</strong>
            <span>不限制物品</span>
          </button>
        ) : null}
        {options.length ? (
          options.map((item) => (
            <button
              className={item.id === value ? "selected" : ""}
              key={item.id}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => selectItem(item)}
              type="button"
            >
              <strong>{itemDisplayName(item)}</strong>
              <span>
                {[
                  item.barcode ? `条码 ${item.barcode}` : null,
                  item.spec,
                  item.unitName,
                ]
                  .filter(Boolean)
                  .join(" · ") || "未设置规格"}
              </span>
            </button>
          ))
        ) : (
          <div className="item-search-empty">没有匹配的物品</div>
        )}
      </div>
    ) : null;

  return (
    <div className="item-search-select">
      <div className="item-search-input-row">
        <input
          aria-label="搜索物品"
          className="table-input item-search-input"
          disabled={disabled}
          ref={inputRef}
          onBlur={() => {
            window.setTimeout(() => setOpen(false), 120);
          }}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
            if (!event.target.value.trim()) {
              onChange("");
            }
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commitCurrentQuery();
            } else if (event.key === "Escape") {
              setOpen(false);
            }
          }}
          placeholder={placeholder}
          value={query}
        />
        {value ? (
          <button
            aria-label="清空物品"
            className="item-search-clear"
            disabled={disabled}
            onMouseDown={(event) => event.preventDefault()}
            onClick={clearSelection}
            type="button"
          >
            x
          </button>
        ) : null}
      </div>
      {menu ? createPortal(menu, document.body) : null}
    </div>
  );
}
