#![allow(dead_code)]
#![allow(unused_variables)]

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
pub mod outline;
pub mod scroll_layout;

pub use ctx::Ctx;
pub use layout::Edit;
pub use md::{LinkInfo, MarkDownImpl, UrlInfo};
pub use cursor::Cursor;
#[allow(unused_imports)] // 对外 re-export，供 `crate::medit::PghCheckBox` 使用
pub use pgh::PghCheckBox;
pub use pgh::{CharRect, CodeInfo, PghText, SegmentType, PghType, PghView, TableInfo, TextSpacing};
pub use undo::{DoItem, DoCmd, DoMngr, MergeRedoAndUndoGuard};
pub use action::{Action, FindCmd, FindReplaceCtx, Trigger};
pub use image::ImageInfo;
pub use outline::{MarkdownOutline, TocCache, TocEntry, TocNode, toc_entries_to_forest};
