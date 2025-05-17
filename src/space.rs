use core::f32;
use std::collections::HashMap;
use std::{fs, vec};
use std::path::PathBuf;
use crate::medit::ctx::EditCfg;
use crate::medit::{IconName, MarkDownImpl, Command};
use crate::ToolBar;
use crate::mem::Config;
use eframe::egui::{collapsing_header, Button, Color32, Frame, Rect, Stroke, Ui, Widget, Window, Vec2, Response, Order};

#[derive(Debug)]
pub struct  RenameWin {
    is_show: bool,
    need_focus: bool,
    org_name: String,
    new_name: String,
}

impl RenameWin {
    pub fn default() -> Self {
        Self {
            is_show: false,
            need_focus: false,
            org_name: String::new(),
            new_name: String::new(),
        }
    }

    fn active(&mut self, name: &str) {
        self.is_show = true;
        self.need_focus = true;
        self.org_name = name.to_string();
        self.new_name = name.to_string();
    }

    fn close(&mut self) {
        self.need_focus = false;
        self.is_show = false;
    }

    //rename window
    //return ture when click rename-button
    fn show(&mut self, ui: &mut Ui) -> bool {
        if self.is_show == false {
            return false;
        }

        let size = Vec2::new(128.0, 30.0);
        let mut rect = Rect::from_min_size(ui.cursor().left_top(), size);
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            rect.min = pointer_pos;
        }
        let mut need_rename = false;
        let title = format!("rename");
        let egui_ctx = ui.ctx();
        Window::new(title)
            .default_rect(rect)
            .open(&mut self.is_show)
            .resizable([false, false])
            .enabled(true)
            .order(Order::TOP)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui|{
                    let r = ui.text_edit_singleline(&mut self.new_name);
                    if self.need_focus {
                        self.need_focus = false;
                        r.request_focus();
                    }
                    if ui.button("rename").clicked() {
                        need_rename = true;
                    }
                });
            });

        if need_rename {
            self.close();
        }

        need_rename
    }
}

pub struct IndexWind {
    pub is_show: bool,
    pub must_at_top: bool,
    pub need_open: Option<String>,
    pub delete_confirm: Option<String>,
    pub is_window :bool,
}

impl IndexWind {
    pub fn default() -> Self {
        Self {
            is_show: false,
            must_at_top: false,
            need_open: None,
            delete_confirm: None,
            is_window: false,
        }
    }
}

impl NoteSpace {
    /// 
    fn comfirm_window(ui: &mut Ui, name: &str) -> Option<bool> {
        let mut ret = None;
        let title = format!("Delete file confirmation");
        let egui_ctx = ui.ctx();

        let size = Vec2::new(128.0, 30.0);
        let mut rect = Rect::from_min_size(ui.cursor().left_top(), size);
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            rect.min = pointer_pos;
        }

        Window::new(&title)
            .resizable([false, false])
            .scroll(false)
            .title_bar(true)
            .default_rect(rect)
            .order(Order::TOP)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui|{
                    ui.label("Are you sure delete");
                    ui.colored_label(Color32::RED, name);
                    ui.label("?");
                    if ui.button("Ok").clicked() {
                        ret = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        ret = Some(false);
                    }
                });
                let layer_id = ui.layer_id();
                egui_ctx.memory_mut(|mem| mem.areas_mut().move_to_top(layer_id));
            });
        
        ret
    }

    fn circle_icon(ui: &mut Ui, _openness: f32, response: &Response) {
        let stroke = ui.style().interact(&response).fg_stroke;
        //let radius = eframe::egui::lerp(2.0..=3.0, openness);
        ui.painter().circle_filled(response.rect.center(), 2.0, stroke.color);
    }
    
    /// return if need open one file
    fn show_sub_index(&mut self, config: &mut Config, ui: &mut Ui, name: &str, deep: usize) -> Option<Command> {
        let mut cmd = None;
        if deep > 10 {
            return cmd;
        }
        let childs = self.get_child_links(name);
        let id = ui.make_persistent_id(name);
        let is_open = config.tree_open_state_is_open(name);
        let show_name = if !is_open && childs.len() > 0 {
            &format!("{}...{}", name, childs.len())
        } else {
            &format!("{}", name)
        };
        let mut state = collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, is_open);
        config.tree_open_state_update(name, state.is_open());
        let header_res = ui.horizontal(|ui|{
            ui.spacing_mut().item_spacing.x = 2.0;

            if childs.len() > 0 {
                state.show_toggle_button(ui, collapsing_header::paint_default_icon);
            } else {
                state.show_toggle_button(ui, Self::circle_icon);
            }
            
            let r = if name == "." {
                ui.label("Note")
            } else {
                Button::new(show_name).fill(Color32::TRANSPARENT).ui(ui)
            };
            if r.clicked() {
                self.index_window.need_open = Some(name.to_string());
                cmd = Some(Command::OpenFile(name.to_string()));
            }

            let is_fiex_in_toolbar = config.fixed_files.contains(&name.to_string());

            //unfixed button
            if is_fiex_in_toolbar {
                if ToolBar::tool_icon_button(ui, IconName::icon_unfixed, false, false, "UnFixed from toolbar").clicked() {
                    cmd = Some(Command::UnFixedFile(name.to_string()));
                }
            }

            if name == "." {
                //new file button
                if ToolBar::tool_icon_button(ui, IconName::icon_new, false, false, "New file").clicked(){
                    if name == "." {
                        cmd = Some(Command::NewFile(None));
                    } else {
                        cmd = Some(Command::NewFile(Some(name.to_string())));
                    }
                }
                //refresh button
                if ToolBar::tool_icon_button(ui, IconName::icon_refresh, false, false, "Refresh index").clicked() {
                    self.flash_data();
                }
            } else {
                //mouse pos is in this line, show tool buttons
                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    let mut line_rect = r.rect;
                    line_rect.set_right(ui.max_rect().right());
                    if line_rect.contains(pointer_pos) && self.index_window.delete_confirm.is_none() {
                        //let frame = line_rect.expand(2.0);
                        //ui.painter().rect_stroke(frame, 3.0, Stroke::new(1.0,ui.style().visuals.selection.bg_fill));

                        //fixed to tool-bar
                        if name != "." && !is_fiex_in_toolbar {
                            if ToolBar::tool_icon_button(ui, IconName::icon_fixed, false, false, "Fixed to toolbar").clicked() {
                                cmd = Some(Command::FixedFile(name.to_string()));
                            }
                        }
                        //new file button
                        if ToolBar::tool_icon_button(ui, IconName::icon_new, false, false, "New file").clicked(){
                            if name == "." {
                                cmd = Some(Command::NewFile(None));
                            } else {
                                cmd = Some(Command::NewFile(Some(name.to_string())));
                            }
                        }
                        //rename file button
                        if ToolBar::tool_icon_button(ui, IconName::icon_file_rename, false, false, "Rename file").clicked() {
                            cmd = Some(Command::RenameFile(name.to_string()));
                        }
                        //delete file button
                        if ToolBar::tool_icon_button(ui, IconName::icon_delete, false, false, "Delete file").clicked() {
                            self.index_window.delete_confirm = Some(name.to_string()); //show comfirm window
                        }
                    }
                }
            }
        });

        state.show_body_indented(&header_res.response, ui, |ui| {
            for c in childs {
                let sub_cmd = self.show_sub_index(config, ui, &c, deep+1);
                if sub_cmd.is_some() {
                    cmd = sub_cmd;
                }
            }
        });
        cmd
    }

    fn show_root_index(&mut self, config: &mut Config, ui: &mut Ui) -> Option<Command> {
        self.show_sub_index(config, ui, ".", 0)
    }

    pub fn show_index_window(&mut self, config: &mut Config, ui: &mut Ui, rect: Rect, outer_rect: Rect) -> Option<Command> {
        let mut cmd = None;
        if self.index_window.is_show == false {
            return None;
        }

        let win_frame = Frame {
            fill: ui.style().visuals.window_fill(),
            rounding: 3.0.into(),
            stroke: Stroke::new(1.0, ui.style().visuals.weak_text_color()),
            outer_margin: 0.0.into(),
            inner_margin: 0.0.into(),
            ..Default::default()
        };
        
        let title = format!("HOME");
        let egui_ctx = ui.ctx();
        let mut is_show = self.index_window.is_show;
        Window::new(title)
            .fixed_rect(rect)
            .constrain_to(outer_rect)
            .open(&mut is_show)
            .resizable([false, false])
            .vscroll(true)
            .title_bar(false)
            .frame(win_frame)
            .show(egui_ctx, |ui| {
                cmd = self.show_root_index(config, ui);
                
                if self.index_window.must_at_top {
                    let layer_id = ui.layer_id();
                    egui_ctx.memory_mut(|mem| mem.areas_mut().move_to_top(layer_id));
                    self.index_window.must_at_top = false;
                }
            });

        //update show flag
        self.index_window.is_show = true;

        cmd
    }

    pub fn show_index_view(&mut self, config: &mut Config, ui: &mut Ui, rect: Rect, outer_rect: Rect) -> Option<Command>{
        config.tree_open_state_changed = false;
        let mut cmd = if self.index_window.is_window {
            self.show_index_window(config, ui, rect, outer_rect)
        } else {
            self.show_root_index(config, ui)
        };

        //comfirm delete window
        if let Some(delete_confirm) = &self.index_window.delete_confirm {
            match Self::comfirm_window(ui, delete_confirm) {
                Some(need_delete) => {
                    if need_delete {
                        cmd = Some(Command::DeleteFile(delete_confirm.clone()));
                    }
                    self.index_window.delete_confirm = None;    //close comfirm window
                }
                _ => {}
            }
        }

        //close this window when need open file
        if self.index_window.is_window {
            match cmd {
                Some(Command::OpenFile(_)) => self.close_index_window(),
                _ => {}
            }
        }

        cmd
    }

    pub fn active_index_window(&mut self) {
        self.index_window.is_show = true;
        self.index_window.must_at_top = true;
    }
    
    pub fn close_index_window(&mut self) {
        self.index_window.must_at_top = false;
        self.index_window.is_show = false;
    }

    pub fn is_show_index_window(&self) -> bool {
        self.index_window.is_show
    }

    pub fn set_show_index_window(&mut self, is: bool) {
        if is {
            self.active_index_window();
        } else {
            self.close_index_window();
        }
    }
}


#[allow(dead_code)]
#[derive(Clone,Debug)]
pub struct DirNote {
    deep: usize,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FilePath {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CurFile {
    Note(FilePath),
    File(FilePath)
}

impl CurFile {
    pub fn from(name: &str) -> Self {
        if name.contains("/") || name.contains("\\")  {
            if let Some(file_name) = PathBuf::from(name).file_name() {
                return CurFile::File(
                    FilePath{
                        name:file_name.to_string_lossy().to_string(),
                        path: name.to_string()
                });
            } 
        }
        return CurFile::Note(
            FilePath{
                name: name.to_string(),
                path: name.to_string()
        });
    }

    pub fn is_note(&self) -> bool {
        match self {
            CurFile::Note(_) => true,
            _ => false,
        }
    }

    pub fn is_file(&self) -> bool {
        !self.is_note()
    }

    pub fn name(&self) -> String {
        return match self {
            CurFile::File(file) => file.name.clone(),
            CurFile::Note(note) => note.name.clone(),
        };
    }

    pub fn path(&self) -> String {
        return match self {
            CurFile::File(file) => file.path.clone(),
            CurFile::Note(note) => note.path.clone(),
        };
    }

    pub fn name4open(&self) -> String {
        return match self {
            CurFile::File(file) => file.path.clone(),
            CurFile::Note(note) => note.name.clone(),
        };
    }
}

pub struct NoteSpace {
    work_dir: PathBuf,
    files: Vec<PathBuf>,
    file_links: HashMap<String, Vec<String>>,
    link_parents: HashMap<String, Vec<String>>,
    directory: Vec<DirNote>,
    cur_file: Option<CurFile>,
    rename_window: RenameWin,
    index_window: IndexWind,
}

/// rename window
impl NoteSpace {
    pub fn rename_window_active(&mut self, name: &str) {
        self.rename_window.active(name)
    }

    pub fn rename_window_show(&mut self, ui: &mut Ui) -> bool {
        self.rename_window.show(ui)
    }

    pub fn rename_from_to(&self) -> (String, String) {
        (self.rename_window.org_name.clone(), self.rename_window.new_name.clone())
    }
}

impl NoteSpace {
    pub fn new() -> Self {
        let mut space = Self {
            work_dir: PathBuf::from("./note"),
            files: vec![],
            file_links: HashMap::new(),
            link_parents: HashMap::new(),
            directory: vec![],
            cur_file: None,
            rename_window: RenameWin::default(),
            index_window: IndexWind::default(),
        };

        space.set_work_dir();
        space.flash_data();
        space
    }

    fn set_work_dir(&mut self) {
        let exe_path = std::env::current_exe().unwrap();
        let mut parent_dir = exe_path.parent().map(|p| p.to_path_buf()).unwrap();
        parent_dir.push("note");
        self.work_dir = parent_dir;

        if std::fs::metadata(self.work_dir.clone()).is_err(){
            let _= std::fs::create_dir(self.work_dir.clone());
        }

        if std::fs::metadata(self.image_path()).is_err(){
            let _= std::fs::create_dir(self.image_path());
        }
    }

    fn set_files_in_word_dir(&mut self) {
        let mut paths = vec![];
        if let Ok(dir) = fs::read_dir(self.work_dir.clone()) {
            for entry in dir{
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                        paths.push(path);
                    }
                }
            }
        }
        self.files = paths;
    }

    //return map of file links
    fn set_file_links(&mut self) {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for file in &self.files {
            let mut links = vec![];
            if let Ok(s) = std::fs::read_to_string(file.clone()) {
                let mut cfg = EditCfg::new(17.0, true, None);
                let markdown = MarkDownImpl::new_simple(&s, &mut cfg);
                links = markdown.markdown_get_links();
            }
            let mut file_name = file.clone();
            file_name.set_extension("");
            let file_name = file_name.file_name().unwrap().to_str().unwrap().to_string();
            map.insert(file_name, links);
        }
        self.file_links = map;
    }

    //return map of link-parents
    fn set_link_parents(&mut self) {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (file, links) in &self.file_links {
            for link in links {
                if let Some(parents) = map.get_mut(link) {
                    if !parents.contains(file) {
                        parents.push(file.to_string());
                    }
                } else {
                    map.insert(link.to_string(), vec![file.to_string()]);
                }
            }
        }
        self.link_parents = map;
    }

    pub fn rebuild_directory(&mut self) {
        let mut list = vec![];
        for child in self.get_root_files() {
            self.sub_directory(&child, 0, &mut list);
        }
        self.directory = list;
    }

    fn sub_directory(&self, name: &str, deep: usize, list: &mut Vec<DirNote>) {
        if deep > 5 {
            return;
        }
        list.push(DirNote{
            deep,
            name: name.to_string()});

        for child in self.get_child_links(name) {
            self.sub_directory(&child, deep+1, list);
        }
    }

    pub fn flash_data(&mut self) {
        self.set_files_in_word_dir();
        self.set_file_links();
        self.set_link_parents();
        self.rebuild_directory();
    }

    pub fn get_path_from_link_parents(&self, name: &str) -> Vec<String> {
        let mut paths = vec![];
        let mut cur_name = name.to_string();

        loop {
            if paths.contains(&cur_name) {
                break;
            }
            paths.insert(0, cur_name.clone());
            if let Some(parents) = self.link_parents.get(&cur_name) {
                if let Some(parent) = parents.first() {
                    cur_name = parent.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        paths
    }

    fn get_root_files(&self) -> Vec<String> {
        let mut roots = vec![];
        for (file, _) in &self.file_links {
            if None == self.link_parents.get(file) {
                roots.push(file.clone());
            }
        }
        roots.sort();
        roots
    }

    pub fn note_name_to_curfile(&self, name: &str) -> CurFile {
        let path = "./".to_string() + &self.get_path_from_link_parents(name).join("/");
        CurFile::Note(FilePath{
            name: name.to_string(), 
            path
        })
    }

    pub fn set_current_file(&mut self, cur_file: &CurFile) {
        self.cur_file = Some(cur_file.clone());
    }

    pub fn get_current_cur(&self) -> Option<CurFile> {
        self.cur_file.clone()
    }

    pub fn get_current_path(&self) -> Option<String> {
        if let Some(cur_file) = &self.cur_file {
            return Some(cur_file.path());
        }
        None
    }

    pub fn get_current_name(&self) -> Option<String> {
        if let Some(cur_file) = &self.cur_file {
            return Some(cur_file.name());
        }
        None
    }

    pub fn get_current_file(&self) -> Option<String> {
        if let Some(cur_file) = &self.cur_file {
            if cur_file.is_file() {
                return Some(cur_file.name());
            }
        }
        None
    }
    
    pub fn get_current_note(&self) -> Option<String> {
        if let Some(cur_file) = &self.cur_file {
            if cur_file.is_note() {
                return Some(cur_file.name());
            }
        }
        None
    }

    pub fn get_child_links(&self, name: &str) -> Vec<String> {
        if let Some(links) = self.file_links.get(name).cloned() {
            return links;
        } else if name == "." {
            return self.get_root_files();
        }
        vec![]
    }

    pub fn get_parents(&self, name: &str) -> Vec<String> {
        if let Some(links) = self.link_parents.get(name).cloned() {
            return links;
        } 
        vec![]
    }

    pub fn name2path(&self, name: &str) -> String {
        format!("{}/{}.md", &self.work_dir.display(), name)
    }

    pub fn is_file_exist(&self, name: &str) -> bool {
        let path = self.name2path(name);
        std::fs::metadata(path).is_ok()
    }

    pub fn new_file_name(&self) -> Option<String> {
        for i in 1..999 {
            let name = format!("untitled_{}", i);
            if self.is_file_exist(&name) == false {
                return Some(name);
            } 
        }
        None
    }

    pub fn rename(&self, org: &str, new: &str) -> std::io::Result<()> {
        let from = self.name2path(org);
        let to = self.name2path(new);
        std::fs::rename(from, to)
    }

    pub fn delete_file(&self, file: &str) -> std::io::Result<()> {
        let from = self.name2path(file);
        std::fs::remove_file(from)
    }

    pub fn read_note(&self, name: &str) -> std::io::Result<String> {
        let path = self.name2path(name);
        std::fs::read_to_string(path)
    }

    pub fn write_note(&self, name: &str, text: &str) -> std::io::Result<()> {
        let path = self.name2path(name);
        std::fs::write(path, text)
    }

    pub fn write_file(&self, path: &str, text: &str) -> std::io::Result<()> {
        std::fs::write(path, text)
    }

    pub fn config_file(&self) -> String {
        format!("{}/{}", &self.work_dir.display(), "config.json")
    }

    pub fn image_path(&self) -> String {
        let path = format!("{}/{}", &self.work_dir.display(), "images");
        path.replace("\\", "/")
    }

    #[allow(dead_code)]
    pub fn get_root_markdown_text(&self) -> String {
        let mut rs = "".to_string();
        for n in &self.directory {
            let mut node_s = "".to_string();
            for _ in 0..=n.deep {
                node_s += "==";
            }
            node_s = node_s + " [[" + &n.name + "]]  \n";
            rs += &node_s;
        }
        rs
    }


}
