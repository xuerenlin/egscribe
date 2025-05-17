#![allow(dead_code)]
#![allow(unused_variables)]

const TEXT_TOP_SPACE: f32 = 1.0;
const TEXT_BOTTOM_SPACE: f32 = 1.0;

pub mod ctx;
pub mod icon;
pub mod items;
pub mod layout;
pub mod md;
pub mod pgh;
pub mod text;
pub mod undo;
pub mod cmd;
pub mod cursor;
pub mod image;

pub use ctx::Ctx;
pub use items::PghCheckBox;
pub use layout::Edit;
pub use md::{LinkInfo, MarkDownImpl};
pub use cursor::Cursor;
pub use pgh::{CharRect, PghItem, SegmentType, PghType, PghView, TableInfo};
pub use text::PghText;
pub use undo::{DoItem, DoCmd, DoMngr};
pub use icon::IconName;
pub use cmd::{FindCmd, FindReplaceCtx, Command};
pub use image::ImageInfo;
