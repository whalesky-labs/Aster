import { invoke } from "@tauri-apps/api/core";
import type { EditorKind, EditorMode } from "../../shared/lib/editorWindows";
import type { EditorWindowController } from "./useEditorWindowController";
import { ItemEditor } from "./ItemEditor";
import { CategoryEditor, DepartmentEditor, SimpleNameEditor, SupplierEditor } from "./MasterDataEditors";
import { BudgetRuleEditor, UserEditor } from "./AdministrationEditors";

export function renderMasterEditorContent({
  controller, editor, id, mode,
}: { controller: EditorWindowController; editor: EditorKind; id?: string; mode: EditorMode }) {
  const {
    budgetRules, categories, departments, enabledCategories, enabledDepartments,
    enabledSuppliers, enabledUnits, isLoading, isSaving, items, periodMonth,
    roles, runEditorAction, suppliers, units, users,
  } = controller;
  if (editor === "item") {
    return (
      <ItemEditor
        categories={enabledCategories}
        disabled={isSaving || isLoading}
        item={items.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "物品已保存" }, () =>
            invoke("save_item", { request }),
          )
        }
        suppliers={enabledSuppliers}
        units={enabledUnits}
      />
    );
  } else if (editor === "department") {
    return (
      <DepartmentEditor
        departments={departments}
        disabled={isSaving || isLoading}
        item={departments.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "部门已保存" }, () =>
            invoke("save_department", { request }),
          )
        }
      />
    );
  } else if (editor === "category") {
    return (
      <CategoryEditor
        categories={categories}
        disabled={isSaving || isLoading}
        item={categories.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "分类已保存" }, () =>
            invoke("save_category", { request }),
          )
        }
      />
    );
  } else if (editor === "unit") {
    return (
      <SimpleNameEditor
        disabled={isSaving || isLoading}
        fallbackSortOrder={units.length + 1}
        item={units.find((item) => item.id === id)}
        label="单位"
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "单位已保存" }, () =>
            invoke("save_unit", { request }),
          )
        }
      />
    );
  } else if (editor === "supplier") {
    return (
      <SupplierEditor
        disabled={isSaving || isLoading}
        item={suppliers.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "供应商已保存" }, () =>
            invoke("save_supplier", { request }),
          )
        }
      />
    );
  } else if (editor === "budget") {
    return (
      <BudgetRuleEditor
        categories={enabledCategories}
        departments={enabledDepartments}
        disabled={isSaving || isLoading}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "预算规则已保存" }, () =>
            invoke("save_budget_rule", { request }),
          )
        }
        periodMonth={periodMonth}
        rule={budgetRules.find((item) => item.id === id)}
      />
    );
  } else if (editor === "user") {
    return (
      <UserEditor
        departments={departments}
        disabled={isSaving || isLoading}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "用户已保存" }, () =>
            invoke("save_user_account", { request }),
          )
        }
        roles={roles}
        user={users.find((item) => item.id === id)}
      />
    );
  }
  return null;
}
