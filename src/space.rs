use core::f32;
use std::collections::{HashMap, HashSet};
use std::{fs, vec};
use std::path::{Path, PathBuf};
use crate::medit::cfg::EditCfg;
use crate::uicom::{IconName, CONTROL_HIGHLIGHT, galley_builder, icon_button_builder};
use crate::medit::{MarkDownImpl, Action, cfg::HeightMode};
use crate::config::Config;
use crate::i18n::tr;
use eframe::egui::{collapsing_header, Button, Color32, Frame, Label, Rect, RichText, Stroke, Ui, Widget, Window, Vec2, Response, Order};

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
        let title = tr("space.rename.title");
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
                    if ui.button(tr("space.rename.button")).clicked() {
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
        let title = tr("space.delete.confirm.title");
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
                    ui.label(tr("space.delete.confirm.message"));
                    ui.colored_label(Color32::RED, name);
                    ui.label("?");
                    if ui.button(tr("space.delete.confirm.ok")).clicked() {
                        ret = Some(true);
                    }
                    if ui.button(tr("space.delete.confirm.cancel")).clicked() {
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

    /// 索引行右键菜单内的条目（固定 / 新建 / 重命名 / 删除）。
    fn index_row_context_menu_items(
        ui: &mut Ui,
        space: &mut NoteSpace,
        config: &Config,
        name: &str,
        cmd: &mut Option<Action>,
    ) {
        let is_fixed = config.fixed_files.contains(&name.to_string());
        let fixed_icon = if is_fixed {
            IconName::icon_unfixed
        } else {
            IconName::icon_fixed
        };
        let fixed_text = if is_fixed {
            tr("space.index.unfix_note.tooltip")
        } else {
            tr("space.index.fix_note.tooltip")
        };
        if Button::new(
            galley_builder(ui)
                .icon(fixed_icon)
                .text(format!(" {}", fixed_text))
                .build(),
        )
        .ui(ui)
        .clicked()
        {
            if is_fixed {
                *cmd = Some(Action::unfixed_file(name.to_string()));
            } else {
                *cmd = Some(Action::fixed_file(name.to_string()));
            }
        }
        if Button::new(
            galley_builder(ui)
                .icon(IconName::icon_new)
                .text(format!(" {}", tr("space.index.new_note.tooltip")))
                .build(),
        )
        .ui(ui)
        .clicked()
        {
            *cmd = Some(Action::new_file(Some(name.to_string())));
        }
        if Button::new(
            galley_builder(ui)
                .icon(IconName::icon_file_rename)
                .text(format!(" {}", tr("space.index.rename_note.tooltip")))
                .build(),
        )
        .ui(ui)
        .clicked()
        {
            *cmd = Some(Action::rename_file(name.to_string()));
        }
        if Button::new(
            galley_builder(ui)
                .icon(IconName::icon_delete)
                .text(format!(" {}", tr("space.index.delete_note.tooltip")))
                .build(),
        )
        .ui(ui)
        .clicked()
        {
            space.index_window.delete_confirm = Some(name.to_string());
        }
    }

    /// 新版：在展开按钮与标题上挂右键上下文菜单。
    fn index_row_attach_context_menu(
        space: &mut NoteSpace,
        config: &Config,
        toggle_resp: Response,
        row_label_resp: Response,
        name: &str,
        cmd: &mut Option<Action>,
    ) {
        if space.index_window.delete_confirm.is_some() {
            return;
        }
        toggle_resp.union(row_label_resp).context_menu(|ui| {
            Self::index_row_context_menu_items(ui, space, config, name, cmd);
        });
    }

    /// 旧版：鼠标悬停在本行时于右侧显示工具栏图标。
    ///
    /// 默认已不再调用；若需恢复旧交互，可在 `show_sub_index` 的非根分支中改调用此函数。
    #[allow(dead_code)]
    fn index_row_attach_hover_toolbar_legacy(
        space: &mut NoteSpace,
        config: &Config,
        ui: &mut Ui,
        name: &str,
        row_label_resp: &Response,
        cmd: &mut Option<Action>,
    ) {
        let Some(pointer_pos) = ui.ctx().pointer_interact_pos() else {
            return;
        };
        let mut line_rect = row_label_resp.rect;
        line_rect.set_right(ui.max_rect().right());
        if !line_rect.contains(pointer_pos) || space.index_window.delete_confirm.is_some() {
            return;
        }

        let is_fixed = config.fixed_files.contains(&name.to_string());
        let fixed_icon = if is_fixed {
            IconName::icon_unfixed
        } else {
            IconName::icon_fixed
        };
        if icon_button_builder(ui)
            .icon(fixed_icon)
            .hover_text(if is_fixed {
                tr("space.index.unfix_note.tooltip")
            } else {
                tr("space.index.fix_note.tooltip")
            })
            .build_tool()
            .clicked()
        {
            if is_fixed {
                *cmd = Some(Action::unfixed_file(name.to_string()));
            } else {
                *cmd = Some(Action::fixed_file(name.to_string()));
            }
        }

        if icon_button_builder(ui)
            .icon(IconName::icon_new)
            .hover_text(tr("space.index.new_note.tooltip"))
            .build_tool()
            .clicked()
        {
            *cmd = Some(Action::new_file(Some(name.to_string())));
        }
        if icon_button_builder(ui)
            .icon(IconName::icon_file_rename)
            .hover_text(tr("space.index.rename_note.tooltip"))
            .build_tool()
            .clicked()
        {
            *cmd = Some(Action::rename_file(name.to_string()));
        }
        if icon_button_builder(ui)
            .icon(IconName::icon_delete)
            .hover_text(tr("space.index.delete_note.tooltip"))
            .build_tool()
            .clicked()
        {
            space.index_window.delete_confirm = Some(name.to_string());
        }
    }
    
    fn show_sub_index(&mut self, config: &mut Config, ui: &mut Ui, name: &str, deep: usize, visited: &mut HashSet<String>) -> Option<Action> {
        let mut cmd = None;
        if deep > 10 {
            return cmd;
        }
        // 检查循环引用：如果当前节点已经在访问路径中，则跳过以避免无限递归
        if visited.contains(name) {
            return cmd;
        }
        // 将当前节点添加到已访问集合中
        visited.insert(name.to_string());
        
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

            let toggle_resp = if childs.len() > 0 {
                state.show_toggle_button(ui, collapsing_header::paint_default_icon)
            } else {
                state.show_toggle_button(ui, Self::circle_icon)
            };

            let r = if config.fixed_files.contains(&name.to_string()) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let name_btn = Button::new(RichText::new(show_name).strong())
                        .fill(Color32::TRANSPARENT)
                        .ui(ui);
                    let pin_galley = galley_builder(ui)
                        .icon(IconName::icon_fixed)
                        .fg(CONTROL_HIGHLIGHT)
                        .build();
                    let pin_lbl = ui.add(Label::new(pin_galley).selectable(false));
                    name_btn.union(pin_lbl)
                })
                .inner
            } else {
                Button::new(RichText::new(show_name))
                    .fill(Color32::TRANSPARENT)
                    .ui(ui)
            };
            if r.clicked() {
                self.index_window.need_open = Some(name.to_string());
                cmd = Some(Action::open_file(name.to_string()));
            }

            // 新版：右键上下文菜单（菜单条目见 `index_row_context_menu_items`）。
            Self::index_row_attach_context_menu(self, config, toggle_resp, r, name, &mut cmd);
            // 旧版悬停工具栏（保留以便回退）：
            // Self::index_row_attach_hover_toolbar_legacy(self, config, ui, name, &r, &mut cmd);
        });

        state.show_body_indented(&header_res.response, ui, |ui| {
            for c in childs {
                let sub_cmd = self.show_sub_index(config, ui, &c, deep+1, visited);
                if sub_cmd.is_some() {
                    cmd = sub_cmd;
                }
            }
        });
        // 递归返回后，从 visited 中移除当前节点，允许在其他路径中再次访问
        visited.remove(name);
        cmd
    }

    fn show_root_index(&mut self, config: &mut Config, ui: &mut Ui) -> Option<Action> {
        let mut cmd = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(tr("space.index.note"));

            // new root note button
            if icon_button_builder(ui)
                .icon(IconName::icon_new)
                .hover_text(tr("space.index.new_note.tooltip"))
                .build_tool()
                .clicked()
            {
                cmd = Some(Action::new_file(None));
            }

            // refresh button
            if icon_button_builder(ui)
                .icon(IconName::icon_refresh)
                .hover_text(tr("space.index.refresh.tooltip"))
                .build_tool()
                .clicked()
            {
                self.flash_data();
            }
        });
        ui.add_space(2.0);

        let mut visited = HashSet::new();
        for c in self.get_child_links(".") {
            let sub_cmd = self.show_sub_index(config, ui, &c, 0, &mut visited);
            if sub_cmd.is_some() {
                cmd = sub_cmd;
            }
        }
        cmd
    }

    pub fn show_index_window(&mut self, config: &mut Config, ui: &mut Ui, rect: Rect, outer_rect: Rect) -> Option<Action> {
        let mut cmd = None;
        if self.index_window.is_show == false {
            return None;
        }

        let win_frame = Frame {
            fill: ui.style().visuals.window_fill(),
            stroke: Stroke::new(1.0, ui.style().visuals.weak_text_color()),
            outer_margin: 0.0.into(),
            inner_margin: 0.0.into(),
            ..Default::default()
        };
        
        let title = tr("space.index.home.title");
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

    pub fn show_index_view(&mut self, config: &mut Config, ui: &mut Ui, rect: Rect, outer_rect: Rect) -> Option<Action>{
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
                        cmd = Some(Action::delete_file(delete_confirm.clone()));
                    }
                    self.index_window.delete_confirm = None;    //close comfirm window
                }
                _ => {}
            }
        }

        //close this window when need open note
        if self.index_window.is_window {
            if let Some(ref cmd_ref) = cmd {
                if cmd_ref.command == "open_file" {
                    self.close_index_window();
                }
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
pub enum UniFile {
    Note(FilePath),
    File(FilePath)
}

impl UniFile {
    pub fn from(name: &str) -> Self {
        if name.contains("/") || name.contains("\\")  {
            if let Some(file_name) = PathBuf::from(name).file_name() {
                return UniFile::File(
                    FilePath{
                        name:file_name.to_string_lossy().to_string(),
                        path: name.to_string()
                });
            } 
        }
        return UniFile::Note(
            FilePath{
                name: name.to_string(),
                path: name.to_string()
        });
    }

    pub fn is_note(&self) -> bool {
        match self {
            UniFile::Note(_) => true,
            _ => false,
        }
    }

    pub fn is_file(&self) -> bool {
        !self.is_note()
    }

    pub fn name(&self) -> String {
        return match self {
            UniFile::File(file) => file.name.clone(),
            UniFile::Note(note) => note.name.clone(),
        };
    }

    pub fn path(&self) -> String {
        return match self {
            UniFile::File(file) => file.path.clone(),
            UniFile::Note(note) => note.path.clone(),
        };
    }

    pub fn name4open(&self) -> String {
        return match self {
            UniFile::File(file) => file.path.clone(),
            UniFile::Note(note) => note.name.clone(),
        };
    }
}


#[derive(Clone,Debug)]
pub struct FileCache {
    pub content: String,
    pub links: HashMap<String, ()>,
}

pub struct NoteSpace {
    work_dir: PathBuf,
    file_cache: HashMap<String, FileCache>,
    link_parents: HashMap<String, Vec<String>>,
    directory: Vec<DirNote>,
    cur_file: Option<UniFile>,
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
            file_cache: HashMap::new(),
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

    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    fn set_work_dir(&mut self) {
        let exe_note_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.join("note")));
        let cur_note_dir = std::env::current_dir().ok().map(|p| p.join("note"));
        let cur_output_note_dir = std::env::current_dir()
            .ok()
            .map(|p| p.join("output").join("note"));

        self.work_dir = if let Some(dir) = exe_note_dir.clone().filter(|dir| dir.exists()) {
            dir
        } else if let Some(dir) = cur_output_note_dir.clone().filter(|dir| dir.exists()) {
            dir
        } else if let Some(dir) = cur_note_dir.clone().filter(|dir| dir.exists()) {
            dir
        } else if let Some(dir) = exe_note_dir {
            let _ = fs::create_dir_all(&dir);
            dir
        } else if let Some(dir) = cur_output_note_dir {
            let _ = fs::create_dir_all(&dir);
            dir
        } else {
            let dir = cur_note_dir.unwrap_or_else(|| PathBuf::from("./note"));
            let _ = fs::create_dir_all(&dir);
            dir
        };

        let _ = fs::create_dir_all(&self.work_dir);
        let _ = fs::create_dir_all(self.image_path());
        let _ = fs::create_dir_all(crate::util::cache_dir(&self.work_dir));
    }

    //return links of the file
    fn get_file_links(&mut self, content: &str) -> HashMap<String, ()> {
        let mut cfg = EditCfg::new(17.0, true, None, HeightMode::fix_max());
        let markdown = MarkDownImpl::new_simple(content, &mut cfg);
        return markdown.markdown_get_links();
    }

    fn flash_file_cache(&mut self) {
        let mut file_cache: HashMap<String, FileCache> = HashMap::new();
        if let Ok(dir) = fs::read_dir(self.work_dir.clone()) {
            for entry in dir{
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    let mut tmp_path = file_path.clone();
                    if file_path.is_file() && file_path.extension().map_or(false, |e| e == "md") {
                        if let Ok(content) = std::fs::read_to_string(file_path) {
                            tmp_path.set_extension("");
                            let file_name = tmp_path.file_name().unwrap().to_string_lossy().to_string();
                            let links = self.get_file_links(&content);
                            file_cache.insert(file_name, FileCache{content, links});
                        }
                    }
                }
            }
        }
        self.file_cache = file_cache;
    }

    fn update_file_cache(&mut self, file_name: &str, content: String) {
        if let Some(cache) = self.file_cache.get_mut(file_name) {
            cache.content = content;
        }
    }

    //return map of link-parents
    fn set_link_parents(&mut self) {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (file, cache) in &self.file_cache {
            for link in cache.links.keys() {
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
        self.flash_file_cache();
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
        for (file, _) in &self.file_cache {
            if None == self.link_parents.get(file) {
                roots.push(file.clone());
            }
        }
        roots.sort();
        roots
    }

    pub fn note_name_to_unifile(&self, name: &str) -> UniFile {
        let path = "./".to_string() + &self.get_path_from_link_parents(name).join("/");
        UniFile::Note(FilePath{
            name: name.to_string(), 
            path
        })
    }

    pub fn set_current_file(&mut self, cur_file: &UniFile) {
        self.cur_file = Some(cur_file.clone());
    }

    pub fn get_current_cur(&self) -> Option<UniFile> {
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
        if let Some(cache) = self.file_cache.get(name) {
            let mut links: Vec<String> = cache.links.keys().cloned().collect();
            links.sort();
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
        // Notes use UTF-8 encoding, can read directly
        std::fs::read_to_string(path)
    }

    pub fn write_note(&mut self, name: &str, text: &str) -> std::io::Result<()> {
        let path = self.name2path(name);
        let result = std::fs::write(path, text);
        if result.is_ok() {
            self.update_file_cache(name, text.to_string());
        }
        result
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
