import type { CreateRuleReq as RustCreateRuleReq } from "../generated/rust-types/CreateRuleReq";
import type { CreateSavedViewReq as RustCreateSavedViewReq } from "../generated/rust-types/CreateSavedViewReq";
import type { CreateSourceReq } from "../generated/rust-types/CreateSourceReq";
import type { DeliveriesListResp as RustDeliveriesListResp } from "../generated/rust-types/DeliveriesListResp";
import type { DeliveryDto as RustDeliveryDto } from "../generated/rust-types/DeliveryDto";
import type { EntriesListResp } from "../generated/rust-types/EntriesListResp";
import type { EntryDto } from "../generated/rust-types/EntryDto";
import type { NotificationTestResp } from "../generated/rust-types/NotificationTestResp";
import type { PatchRuleReq as RustPatchRuleReq } from "../generated/rust-types/PatchRuleReq";
import type { PatchSavedViewReq as RustPatchSavedViewReq } from "../generated/rust-types/PatchSavedViewReq";
import type { PatchSourceReq as RustPatchSourceReq } from "../generated/rust-types/PatchSourceReq";
import type { RefreshSourceResp as RustRefreshSourceResp } from "../generated/rust-types/RefreshSourceResp";
import type { RuleDto as RustRuleDto } from "../generated/rust-types/RuleDto";
import type { RulePreviewReq as RustRulePreviewReq } from "../generated/rust-types/RulePreviewReq";
import type { RulesListResp as RustRulesListResp } from "../generated/rust-types/RulesListResp";
import type { SavedViewDto as RustSavedViewDto } from "../generated/rust-types/SavedViewDto";
import type { SavedViewsListResp as RustSavedViewsListResp } from "../generated/rust-types/SavedViewsListResp";
import type { SourceDto } from "../generated/rust-types/SourceDto";
import type { SourcesListResp } from "../generated/rust-types/SourcesListResp";
import type { TestSourceReq } from "../generated/rust-types/TestSourceReq";
import type { TestSourceResp } from "../generated/rust-types/TestSourceResp";

export type MatchScope = "title" | "title_summary";
export type DeliveryStatus = "pending" | "submitting" | "accepted" | "cancelled";

export type {
  CreateSourceReq,
  EntriesListResp,
  EntryDto,
  SourceDto,
  SourcesListResp,
  TestSourceReq,
  TestSourceResp,
};

export type CreateRuleReq = Omit<RustCreateRuleReq, "matchScope"> & {
  matchScope: MatchScope;
};

export type PatchSourceReq = Partial<{
  [Key in keyof RustPatchSourceReq]: NonNullable<RustPatchSourceReq[Key]>;
}>;

export type PatchRuleReq = Partial<{
  [Key in keyof RustPatchRuleReq]: Key extends "matchScope"
    ? MatchScope
    : NonNullable<RustPatchRuleReq[Key]>;
}>;

export type RefreshSourceResp = Omit<RustRefreshSourceResp, "status"> & {
  status: "started" | "already-running";
};

export type RuleDto = Omit<RustRuleDto, "matchScope"> & {
  matchScope: MatchScope;
};

export type RulesListResp = Omit<RustRulesListResp, "rules"> & {
  rules: RuleDto[];
};

export type RulePreviewReq = Omit<
  RustRulePreviewReq,
  "name" | "enabled" | "matchScope" | "cursor"
> & {
  name?: string | null;
  enabled?: boolean;
  matchScope: MatchScope;
  cursor?: string | null;
};

export type RulePreviewResp = EntriesListResp;

export type DeliveryDto = Omit<RustDeliveryDto, "status"> & {
  status: DeliveryStatus;
};

export type DeliveriesListResp = Omit<RustDeliveriesListResp, "deliveries"> & {
  deliveries: DeliveryDto[];
};

export type TestNotificationResp = Omit<NotificationTestResp, "status"> & {
  status: "accepted";
};

export type SavedViewDto = Omit<RustSavedViewDto, "matchScope"> & {
  matchScope: MatchScope;
};

export type CreateSavedViewReq = Omit<
  RustCreateSavedViewReq,
  "matchScope"
> & {
  matchScope: MatchScope;
};

export type PatchSavedViewReq = Partial<{
  [Key in keyof RustPatchSavedViewReq]: Key extends "matchScope"
    ? MatchScope
    : NonNullable<RustPatchSavedViewReq[Key]>;
}>;

export type SavedViewsListResp = Omit<RustSavedViewsListResp, "views"> & {
  views: SavedViewDto[];
};

export interface EntriesQuery {
  q?: string;
  sourceId?: string;
  category?: string;
  author?: string;
  publishedFrom?: string;
  publishedTo?: string;
  matchScope?: MatchScope;
  viewId?: string;
  cursor?: string;
}
