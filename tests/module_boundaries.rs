use rust_skgen::{
    cli, discover, domain, extract, fetch, metadata, policy, render, service, storage,
};

#[test]
fn planned_module_contracts_are_public() {
    let _: Option<&dyn cli::CommandHandler> = None;
    let _: Option<&dyn domain::SkillModel> = None;
    let _: Option<&dyn fetch::DocumentFetcher> = None;
    let _: Option<&dyn policy::CrawlPolicy> = None;
    let _: Option<&dyn discover::SiteDiscoverer> = None;
    let _: Option<&dyn extract::DocumentExtractor> = None;
    let _: Option<&dyn render::SkillRenderer> = None;
    let _: Option<&dyn metadata::MetadataStore> = None;
    let _: Option<&dyn storage::SkillStorage> = None;
    let _: Option<&dyn service::SkillService> = None;
}
