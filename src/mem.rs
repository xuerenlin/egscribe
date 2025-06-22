use serde::{Serialize, Deserialize};
use crate::sitter;
use crate::space::{UniFile, NoteSpace};
use crate::medit::{Command, Ctx, FindCmd, FindReplaceCtx, Cursor};
use crate::find::FindWindow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::usize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub show_line_no: bool,
    pub show_index_window: bool,
    pub wrap: bool,
    pub font_size: f32,
    pub indent_size: f32,
    pub dark_mode: bool,
    pub current_file: String,
    pub fixed_files: Vec<String>,
    pub opend_files: Vec<String>,
    pub tree_open_state: HashMap<String, bool>,
    pub tree_open_state_changed: bool
}

impl Config {
    pub fn default() -> Self {
        Self {
            show_line_no: true,
            show_index_window: true,
            wrap: false,
            font_size: 16.0,
            indent_size: 16.0,
            dark_mode: true,
            current_file: String::new(),
            fixed_files: vec![],
            opend_files: vec![],
            tree_open_state: HashMap::new(),
            tree_open_state_changed: false
        }
    }

    pub fn tree_open_state_update(&mut self, name: &str, is_open: bool) {
        if let Some(old) = self.tree_open_state.insert(name.to_string(), is_open) {
            if old == is_open {
                return;
            }
        }
        self.tree_open_state_changed = true;
    }

    pub fn tree_open_state_is_open(&self, name: &str) -> bool {
        if let Some(is_open) = self.tree_open_state.get(name) {
            *is_open
        } else {
            true
        }
    }
}


pub struct ToolBarInfo {
    pub width: Option<f32>,
    pub is_show_bottom: bool,
}

impl ToolBarInfo {
    pub fn default() -> Self {
        Self {
            width: None,
            is_show_bottom: false,
        }
    }
}

pub struct Store {
    pub config: Config,
    pub ectx_map: HashMap<UniFile, Ctx>,
    pub note_space: NoteSpace,
    pub tool_bar_info: ToolBarInfo,
    pub find_window: FindWindow,
}

impl Store {
    pub fn default() -> Self {
        let mut store = Self {
            ectx_map: HashMap::new(),
            note_space: NoteSpace::new(),
            config: Config::default(),
            tool_bar_info: ToolBarInfo::default(),
            find_window: FindWindow::new(),
        };
        store.config_restore();
        store
    }
    
    pub fn cur_edit_ctx_mut(&mut self) -> Option<(UniFile, &mut Ctx)> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                return Some((curfile, ctx))
            }
        }
        None
    }

    pub fn is_cur_content_changed(&self) -> bool {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get(&curfile) {
                return ctx.is_content_changed();
            }
        }
        false
    }

    fn set_edit_cfg(config: &Config, edit_ctx: &mut Ctx) {
        edit_ctx.cfg_mut().show_line_no = config.show_line_no;
        edit_ctx.cfg_mut().wrap = config.wrap;
        edit_ctx.cfg_mut().dark_mode = config.dark_mode;
        edit_ctx.set_font_size(config.font_size);
        edit_ctx.set_indent_size(config.indent_size);
    }

    pub fn open_set_ctx(&mut self, curfile: &UniFile) {
        if let Some(edit_ctx) = self.ectx_map.get_mut(&curfile) {
            edit_ctx.set_open_time();
            Self::set_edit_cfg(&self.config, edit_ctx);
            self.note_space.set_current_file(&curfile);
            self.config_set_current_file(&curfile);   
        }
    }

    pub fn open_note(&mut self, name: &str) -> std::io::Result<String> {
        let curfile = self.note_space.note_name_to_unifile(name);

        // file isn't exist, create first
        if !self.note_space.is_file_exist(name) {
            self.note_space.write_note(&name, "")?;
            self.note_space.flash_data();
        }
        
        // remove firstly
        let mut old_notes = vec![];
        for (note, _) in &self.ectx_map {
            if note.is_note() {
                old_notes.push(note.clone());
            }
        }
        for note in old_notes {
            self.ectx_map.remove(&note);
        }

        // insert new ctx
        let text = self.note_space.read_note(name)?;
        let new_ctx = Ctx::new(&text, true, Some(self.note_space.image_path()));
        self.ectx_map.insert(curfile.clone(), new_ctx);

        // set ctx
        self.open_set_ctx(&curfile);

        Ok(String::new())
    }  

    pub fn open_file(&mut self, name: &str) -> std::io::Result<String> {
        let curfile = UniFile::from(name);
        let text = std::fs::read_to_string(name)?;
        
        // check new ctx
        if self.ectx_map.get(&curfile).is_none() {
            let mut new_ctx = Ctx::new(&text, false, None);
            if let Some(ext) = PathBuf::from(name).extension(){
                let ext = ext.to_string_lossy().to_string();
                new_ctx.set_height_lang(sitter::ext_to_lang(&ext));
            }
            self.ectx_map.insert(curfile.clone(), new_ctx);
        }
        
        // set ctx
        self.open_set_ctx(&curfile);

        Ok(String::new())
    } 

    /// filename - open filename.md in note space
    /// path - open file in file system
    pub fn open(&mut self, name: &str) -> std::io::Result<String> {
        if name == "" {
            return Ok(String::new())
        }
        if name == "." {
            return Ok(String::new())
        } else if name.contains("/") || name.contains("\\") {
            let new_name = name.replace("\\", "/");
            self.open_file(&new_name)
        } else {
            self.open_note(name)
        }
    }

    pub fn open_unifile(&mut self, unifile: &UniFile) -> std::io::Result<String> {
        match unifile {
            UniFile::File(_) => {
                self.open_file(&unifile.path())
            }
            UniFile::Note(_) => {
                self.open_note(&unifile.name())
            }
        }
    }

    pub fn close(&mut self, file: &UniFile) {
        log::debug!("close {:?}", file);
        if self.ectx_map.len() > 1 {
            // remove firstly
            self.ectx_map.remove(file);

            let last_file = self.ectx_map.iter().max_by(|x, y|{
                let time1 = x.1.get_open_time();
                let time2 = y.1.get_open_time();
                time1.cmp(&time2)
            });
            if let Some((last_file,_)) = last_file {
                log::debug!("open {:?}", last_file);
                self.open_set_ctx(&last_file.clone());
            }
        }
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                let text = ctx.get_all_text();
                if curfile.is_file() {
                    self.note_space.write_file(&curfile.path(), &text)?;
                } else {
                    self.note_space.write_note(&curfile.name(), &text)?;
                }
                ctx.clean_change_tick();
            }
        }
        Ok(())
    }

    pub fn new_note(&mut self, parent: Option<String>) -> std::io::Result<()> {
        if let Some(new_name) = self.note_space.new_file_name() {
            //create new file
            self.note_space.write_note(&new_name, "")?;

            //add link to parent
            if let Some(parent_name) = parent {
                if Some(parent_name.to_string()) == self.note_space.get_current_note() {
                    let _ = self.save();
                } 
                
                let text = self.note_space.read_note(&parent_name)?;
                let text = text + "\n\n[[" + &new_name + "]]";
                self.note_space.write_note(&parent_name, &text)?;
            }
            //flash data
            self.note_space.flash_data();

            //open new file
            self.open(&new_name)?;
        }
        Ok(())
    }

    pub fn rename_file(&mut self, org_name: &str, new_name: &str) -> std::io::Result<()> {
        self.note_space.rename(org_name, new_name)?;

        for parent in self.note_space.get_parents(org_name) {
            //change line content in parent file
            let text = self.note_space.read_note(&parent)?;
            let org_links = format!("[[{}]]", org_name);
            let new_links = format!("[[{}]]", new_name);
            let new_text = text.replace(&org_links, &new_links);
            self.note_space.write_note(&parent, &new_text)?;
        }
        //flash data
        self.note_space.flash_data();

        //open new file
        self.open(new_name)?;
        Ok(())
    }

    pub fn delete_file(&mut self, file: &str) -> std::io::Result<()> {
        self.note_space.delete_file(file)?;
        let mut to_open= "help".to_string();

        for parent in self.note_space.get_parents(file) {
            //change line content in parent file
            let text = self.note_space.read_note(&parent)?;
            let org_links = format!("[[{}]]\n", file);
            let new_text = text.replace(&org_links, "");

            let org_links = format!("[[{}]]", file);
            let new_text = new_text.replace(&org_links, "");
            self.note_space.write_note(&parent, &new_text)?;

            to_open = parent;
        }

        //unfixed from tool-bar
        self.config.fixed_files.retain(|f| *f != file);

        //flash data
        self.note_space.flash_data();

        //open parent file
        self.open(&to_open)?;
        Ok(())
    }

    pub fn execute_goto(&mut self, line_text: String, dead_loop: usize) {
        if dead_loop > 1 {
            return;
        }
        if let Some(find_file) = self.find_window.find_file.clone() {
            let arr: Vec<&str> = line_text.trim().split('.').collect();
            if arr.len() < 3 {
                return;
            }
            let curosr:Cursor = (arr[0].parse::<usize>().unwrap(), arr[1].parse::<usize>().unwrap(), arr[2].parse::<usize>().unwrap()).into();
            if let Some((uni_file,cur_edit)) = self.cur_edit_ctx_mut() {
                if find_file == uni_file {
                    cur_edit.set_cursor2(curosr);
                    cur_edit.set_cursor1_reset();
                } else {
                    if self.open_unifile(&find_file).is_ok() {
                        self.execute_goto(line_text, dead_loop+1);
                    }
                }
            }
        }
    }
    
    pub fn execute_find(&mut self, find: FindReplaceCtx) {
        self.execute_cmd(Command::FindReplace(find));
        let dark_mode = self.config.dark_mode;
        
        let (uni_file, find_cache, find_param) = if let Some((uni_file, edit_ctx)) = self.cur_edit_ctx_mut() {
            let (find_cache, find_param) = edit_ctx.get_find_cache();
               (uni_file, find_cache.clone(), find_param.clone())
        } else {
            return;
        };
        self.find_window.set_find_result(uni_file, &find_cache, &find_param, dark_mode);
    }

    pub fn execute_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::OpenFile(file) => {
                let _ = self.open(&file);
            }
            Command::PathList(parent) => {
                let links = self.note_space.get_child_links(&parent);
                log::debug!("{:?}", links);
            }
            Command::DeleteFile(file) => {
                let _= self.delete_file(&file);
            }
            Command::NewFile(parent) => {
                let _= self.new_note(parent);
            }
            Command::RenameFile(file) => {
                self.note_space.rename_window_active(&file);
            }
            Command::FixedFile(file) => {
                self.config_fixed_file(file);
            }
            Command::UnFixedFile(file) => {
                self.config_unfixed_file(file);
            }
            Command::ClickEditLine(line) => {
                self.execute_goto(line, 0);
            }
            Command::OpenUrl(_url) => {
            }
            Command::FindReplace(mut param) => {
                param.regex_build();
                if let Some((_unifile, edit_ctx)) = self.cur_edit_ctx_mut() {
                    if let Some(find_cmd) = param.cmd.clone() {
                        match find_cmd {
                            FindCmd::Find => {
                                edit_ctx.find_and_select(&param);
                            },
                            FindCmd::Replace => {
                                if edit_ctx.is_selected() {
                                    edit_ctx.insert(param.replace.clone());
                                }
                                edit_ctx.find_and_select(&param);
                            },
                            FindCmd::ReplaceAll => {
                                while edit_ctx.find_and_select(&param) {
                                    edit_ctx.insert(param.replace.clone());
                                }
                            },
                            FindCmd::FindAll => {
                                edit_ctx.find_all(&param);
                                self.tool_bar_info.is_show_bottom = true;
                            },
                        }   
                    }
                }
            }
        }
    }

    pub fn config_save(&self) {
        let json_str = serde_json::to_string_pretty(&self.config).unwrap();
        let config_file = self.note_space.config_file();
        let _ = std::fs::write(&config_file, json_str);
    }

    fn config_update_opend_files(&mut self) {
        let mut opend_files = vec![];
        for (curfile, _) in &self.ectx_map {
            opend_files.push(curfile.name4open());
        }
        self.config.opend_files = opend_files;
    }

    pub fn config_fixed_file(&mut self, file: String) {
        if !self.config.fixed_files.contains(&file) {
            self.config.fixed_files.push(file);
            self.config_save();
        }
    }

    pub fn config_unfixed_file(&mut self, file: String) {
        if self.config.fixed_files.contains(&file) {
            self.config.fixed_files.retain(|f| *f != file);
            self.config_save();
        }
    }

    pub fn config_set_current_file(&mut self, curfile: &UniFile) {
        if curfile.is_file() {
            self.config.current_file = curfile.path();
        } else {
            self.config.current_file = curfile.name();
        }
        self.config_update_opend_files();
        self.config_save();
    }

    pub fn config_switch_wrap_mode(&mut self) {
        self.config.wrap = !self.config.wrap;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().wrap = self.config.wrap;
        }
        self.config_save();
    }

    pub fn config_switch_show_line_no(&mut self) {
        self.config.show_line_no = !self.config.show_line_no;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().show_line_no = self.config.show_line_no;
        }
        self.config_save();
    }

    pub fn config_update_dark_mode(&mut self, dark_mode: bool) {
        self.config.dark_mode = dark_mode;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.update_view_mode(self.config.dark_mode);
        }
        self.config_save();
    }

    pub fn config_set_font_size(&mut self, size: f32) {
        self.config.font_size = size;
        if self.config.font_size < 6.0 {
            self.config.font_size = 6.0
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_font_size(size as f32);
        }
        self.config_save();
    }

    pub fn config_add_indent_size(&mut self, delta: f32) {
        self.config.indent_size += delta;
        if self.config.indent_size < 0.0 || self.config.indent_size > 100.0 {
            self.config.indent_size = 0.0
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_indent_size(self.config.indent_size);
        }
        self.config_save();
    }

    pub fn config_update_show_index_window(&mut self, is_show: bool) {
        self.config.show_index_window = is_show;
        self.note_space.set_show_index_window(is_show);
        self.config_save();
    }

    pub fn config_restore(&mut self) {
        let config_file = self.note_space.config_file();
        if let Ok(json_str) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<Config>(&json_str) {
                self.config = config;
            }
        }
        self.note_space.set_show_index_window(self.config.show_index_window);

        //restore current file
        if self.config.current_file.is_empty() {
            let curfile = self.note_space.note_name_to_unifile("untitled_1");
            self.config_set_current_file(&curfile);
        }
        let current_file = self.config.current_file.clone();

        //restore opend files
        for file in self.config.opend_files.clone() {
            let _= self.open(&file);
        }
        let _= self.open(&current_file);
    }
}
