//! 下载任务命令域。

mod arguments;
mod control;
mod files;
mod lifecycle;
mod model;
mod output;
pub(crate) mod parser;

pub use control::*;
pub use files::*;
pub use lifecycle::*;
pub use model::DownloadState;
pub(crate) use output::atomic_move;
