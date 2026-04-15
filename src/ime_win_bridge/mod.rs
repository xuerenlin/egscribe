//! Windows IME bridge module.
//!
//! This module hosts Windows-specific IME integration (`IMM` + `TSF`) used by
//! the editor to avoid accidental text deletion around IME preedit/candidate
//! lifecycle boundaries.

pub mod os_win;
pub mod tsf_win;
