import type {
  CreateRuleReq,
  CreateSavedViewReq,
  CreateSourceReq,
  DeliveriesListResp,
  EntriesListResp,
  EntriesQuery,
  EntryDto,
  PatchRuleReq,
  PatchSavedViewReq,
  PatchSourceReq,
  RefreshSourceResp,
  RuleDto,
  RulePreviewReq,
  RulePreviewResp,
  RulesListResp,
  SavedViewDto,
  SavedViewsListResp,
  SourceDto,
  SourcesListResp,
  TestNotificationResp,
  TestSourceReq,
  TestSourceResp,
} from "./types";

const API_BASE = "/api/apps/rss";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

async function errorMessage(response: Response): Promise<string> {
  const fallback = `${response.status} ${response.statusText}`.trim();
  const text = await response.text();
  if (!text) return fallback;
  try {
    const parsed: unknown = JSON.parse(text);
    if (isRecord(parsed) && typeof parsed.error === "string") return parsed.error;
  } catch {
    return text;
  }
  return text;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    credentials: "include",
    ...init,
    headers: {
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
      ...init?.headers,
    },
  });
  if (!response.ok) throw new Error(await errorMessage(response));
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

function withQuery(path: string, query: EntriesQuery): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (!value) continue;
    if (key === "publishedFrom" && /^\d{4}-\d{2}-\d{2}$/.test(value)) {
      params.set(key, `${value}T00:00:00.000Z`);
    } else if (key === "publishedTo" && /^\d{4}-\d{2}-\d{2}$/.test(value)) {
      params.set(key, `${value}T23:59:59.999Z`);
    } else {
      params.set(key, value);
    }
  }
  const encoded = params.toString();
  return encoded ? `${path}?${encoded}` : path;
}

function json(method: "POST" | "PATCH", body: unknown): RequestInit {
  return { method, body: JSON.stringify(body) };
}

export const api = {
  sources: {
    list: (): Promise<SourcesListResp> => request("/sources"),
    create: (input: CreateSourceReq): Promise<SourceDto> =>
      request("/sources", json("POST", input)),
    patch: (id: string, input: PatchSourceReq): Promise<SourceDto> =>
      request(`/sources/${encodeURIComponent(id)}`, json("PATCH", input)),
    test: (input: TestSourceReq): Promise<TestSourceResp> =>
      request("/sources/test", json("POST", input)),
    refresh: (id: string): Promise<RefreshSourceResp> =>
      request(`/sources/${encodeURIComponent(id)}/refresh`, json("POST", {})),
  },
  entries: {
    list: (query: EntriesQuery): Promise<EntriesListResp> =>
      request(withQuery("/entries", query)),
    get: (id: string): Promise<EntryDto> =>
      request(`/entries/${encodeURIComponent(id)}`),
  },
  rules: {
    list: (): Promise<RulesListResp> => request("/rules"),
    create: (input: CreateRuleReq): Promise<RuleDto> =>
      request("/rules", json("POST", input)),
    patch: (id: string, input: PatchRuleReq): Promise<RuleDto> =>
      request(`/rules/${encodeURIComponent(id)}`, json("PATCH", input)),
    delete: (id: string): Promise<void> =>
      request(`/rules/${encodeURIComponent(id)}`, { method: "DELETE" }),
    preview: (input: RulePreviewReq): Promise<RulePreviewResp> =>
      request("/rules/preview", json("POST", input)),
  },
  views: {
    list: (): Promise<SavedViewsListResp> => request("/views"),
    create: (input: CreateSavedViewReq): Promise<SavedViewDto> =>
      request("/views", json("POST", input)),
    patch: (id: string, input: PatchSavedViewReq): Promise<SavedViewDto> =>
      request(`/views/${encodeURIComponent(id)}`, json("PATCH", input)),
    delete: (id: string): Promise<void> =>
      request(`/views/${encodeURIComponent(id)}`, { method: "DELETE" }),
  },
  deliveries: {
    list: (): Promise<DeliveriesListResp> => request("/deliveries"),
  },
  notifications: {
    test: (): Promise<TestNotificationResp> =>
      request("/notifications/test", json("POST", {})),
  },
};
