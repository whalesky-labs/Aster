export type Role = { id: string; code: string; name: string };

export type UserAccount = {
  id: string;
  username: string;
  displayName: string;
  email?: string | null;
  departmentId?: string | null;
  departmentName?: string | null;
  enabled: boolean;
  roles: Role[];
  createdAt: string;
  updatedAt: string;
};

export type CurrentUser = {
  id: string;
  username: string;
  displayName: string;
  departmentId?: string | null;
  departmentName?: string | null;
  roles: Role[];
  permissions: string[];
};
