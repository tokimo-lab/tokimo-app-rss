//! References every API DTO so `cargo test` exports the camelCase TypeScript contract.
#![allow(unused_imports)]
use tokimo_app_rss::handlers::{
    CreateRuleReq, CreateSavedViewReq, CreateSourceReq, DeleteResp, DeliveriesListResp, DeliveryDto, EntriesListResp,
    EntryDto, NotificationTestResp, PatchRuleReq, PatchSavedViewReq, PatchSourceReq, RefreshSourceResp, RuleDto,
    RulePreviewReq, RulePreviewResp, RulesListResp, SavedViewDto, SavedViewsListResp, SourceDto, SourcesListResp,
    TestSourceReq, TestSourceResp,
};
