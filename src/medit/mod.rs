const TEXT_TOP_SPACE: f32 = 1.0;
const TEXT_BOTTOM_SPACE: f32 = 1.0;

pub mod ctx;
pub mod cfg;
pub mod layout;
pub mod md;
pub mod pgh;
pub mod undo;
pub mod cursor;
pub mod image;
pub mod ctxmenu;
pub mod action;

pub use ctx::Ctx;
#[allow(unused_imports)]
pub use ctx::ScrollToLineMode;
pub use layout::Edit;
pub use md::{LinkInfo, MarkDownImpl, UrlInfo};
pub use cursor::Cursor;
#[allow(unused_imports)] // 对外 re-export，供 `crate::medit::PghCheckBox` 使用
pub use pgh::PghCheckBox;
pub use pgh::{CharRect, CodeInfo, CodeKey, PghText, SegmentType, PghType, PghView, TableInfo, TableKey, TextSpacing};
pub use undo::{DoItem, DoCmd, DoMngr, MergeRedoAndUndoGuard};
pub use action::{Action, FindCmd, FindReplaceCtx, Trigger};
pub use image::ImageInfo;
#[allow(unused_imports)] // 对外 re-export，供外部按需使用 TocCache 等类型
pub use ctx::cache_outline::{TocCache, TocEntry, TocNode, toc_entries_to_forest};
