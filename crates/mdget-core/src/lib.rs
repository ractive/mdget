mod crawl;
mod extract;
mod fetch;
mod filename;
mod links;
mod normalize;
pub(crate) mod robots;
pub(crate) mod sitemap;

pub use crawl::{CrawlOptions, CrawlProgress, CrawlResult, crawl};
pub use extract::{
    ExtractOptions, ExtractResult, Metadata, extract, format_metadata_frontmatter, strip_images,
    truncate_output, word_count,
};
pub use fetch::{FetchOptions, FetchResult, fetch, read_local};
pub use filename::generate_filename;
pub use links::{extract_links, is_static_asset_url};
pub use normalize::normalize_url;
