pub mod diff;
pub mod line_data;
pub mod merger;
pub mod source_data;

pub use diff::{Diff, DiffList, DiffRange, SrcSelector};
pub use line_data::LineData;
pub use merger::{MergeBlock, MergeDetails, Merger, ResolvedChoice};
pub use source_data::SourceData;
