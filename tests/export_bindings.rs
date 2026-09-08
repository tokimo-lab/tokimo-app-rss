//! References every API DTO so `cargo test` exports the camelCase TypeScript contract.
#![allow(unused_imports)]
use tokimo_app_rss::handlers::{
    CreateRuleReq, CreateSourceReq, DeleteResp, DeliveriesListResp, DeliveryDto, EntriesListResp, EntryDto,
    NotificationTestResp, PatchRuleReq, PatchSourceReq, RefreshSourceResp, RuleDto, RulePreviewReq, RulePreviewResp,
    RulesListResp, SourceDto, SourcesListResp, TestSourceReq, TestSourceResp,
};
