export type ApprovalRequest = {
  id: string;
  entityType: string;
  entityId: string;
  status: string;
  requestedBy?: string | null;
  decidedBy?: string | null;
  reason?: string | null;
  decisionNote?: string | null;
  createdAt: string;
  decidedAt?: string | null;
};
