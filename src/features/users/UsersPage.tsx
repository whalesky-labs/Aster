import { useMemo, useState } from "react";
import type { CurrentUser, UserAccount } from "../../entities/users";
import { matchesSearchText } from "../../shared/lib/display";
import { EmptyRow, Status, TableFeatureToolbar, TableSearchToolbar } from "../../shared/ui/DataTable";

export function UsersPage({
  currentUser,
  onCreate,
  onEdit,
  onToggle,
  users,
}: {
  currentUser: CurrentUser | null;
  onCreate: () => void;
  onEdit: (userId: string) => void;
  onToggle: (userId: string, enabled: boolean) => Promise<void>;
  users: UserAccount[];
}) {
  const isAdmin = currentUser?.roles.some((role) => role.code === "admin") ?? false;
  const [search, setSearch] = useState("");
  const filteredUsers = useMemo(
    () => users.filter((user) => matchesSearchText(search, [user.username, user.displayName, user.email, user.departmentName, user.roles.map((role) => role.name).join(" "), user.enabled ? "启用" : "停用"])),
    [search, users],
  );

  if (!isAdmin) {
    return (
      <section className="module-panel placeholder-panel">
        <h2>需要管理员权限</h2>
        <p>用户管理、角色分配和危险操作控制需要管理员登录后使用。</p>
      </section>
    );
  }

  return (
    <section className="table-panel">
      <TableFeatureToolbar action={<button className="primary-button" onClick={onCreate} type="button">新增用户</button>} />
      <TableSearchToolbar onSearchChange={setSearch} placeholder="用户名、显示名称、邮箱、部门、角色" search={search} />
      <table>
        <thead><tr><th>用户名</th><th>显示名称</th><th>邮箱</th><th>所属部门</th><th>角色</th><th>状态</th><th>操作</th></tr></thead>
        <tbody>
          {filteredUsers.map((user) => (
            <tr key={user.id}>
              <td>{user.username}</td><td>{user.displayName}</td><td>{user.email ?? "-"}</td><td>{user.departmentName ?? "-"}</td>
              <td>{user.roles.map((role) => role.name).join("、") || "-"}</td><td><Status enabled={user.enabled} /></td>
              <td className="row-actions"><button onClick={() => onEdit(user.id)}>编辑</button><button onClick={() => onToggle(user.id, !user.enabled)}>{user.enabled ? "停用" : "启用"}</button></td>
            </tr>
          ))}
          {filteredUsers.length === 0 ? <EmptyRow colSpan={7} /> : null}
        </tbody>
      </table>
    </section>
  );
}
