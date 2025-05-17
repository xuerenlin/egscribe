
use regex::{Regex, RegexBuilder};

#[derive(Clone)]
pub enum FindCmd {
    Find,
    Replace,
    ReplaceAll,
    FindAll,
}

#[derive(Clone)]
pub struct FindReplaceCtx {
    pub find: String,
    pub replace: String,
    pub is_case: bool,
    pub is_hole_word: bool,
    pub is_reg: bool,
    pub cmd: Option<FindCmd>,
    pub regex: Option<Regex>,
}

impl FindReplaceCtx {
    pub fn new() -> Self {
        FindReplaceCtx {
            find: "".to_string(),
            replace: "".to_string(),
            is_case: false,
            is_hole_word: false,
            is_reg: false,
            cmd: None,
            regex: None,
        }
    }

    pub fn sample(find: String) -> Self {
        let mut s = FindReplaceCtx::new();
        s.find = find;
        s.is_case = true;
        s
    }

    pub fn regex_build(&mut self) {
        if self.is_reg {
            let mut builder = RegexBuilder::new(&self.find);
            builder.case_insensitive(!self.is_case);
            if let Ok(re) = builder.build() {
                self.regex = Some(re);
                return;
            }
        }
        self.regex = None;
    }
}

pub enum Command {
    OpenFile(String),
    PathList(String),
    DeleteFile(String),
    NewFile(Option<String>),    //Option<parent>
    RenameFile(String),   
    FixedFile(String),   
    UnFixedFile(String),   
    FindReplace(FindReplaceCtx),   
    ClickEditLine(String),
    OpenUrl(String),
}
