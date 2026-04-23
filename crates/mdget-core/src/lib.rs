mod extract;
mod fetch;
mod filename;

pub use extract::{
    ExtractOptions, ExtractResult, Metadata, extract, format_metadata_frontmatter, strip_images,
    truncate_output, word_count,
};
pub use fetch::{FetchOptions, FetchResult, fetch, read_local};
pub use filename::generate_filename;
