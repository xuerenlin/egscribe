use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use super::index::IndexCache;
use super::Ctx;
use crate::medit::CodeKey;

/// 代码块元数据（按 `code_key` 索引）。
#[derive(Clone, Debug, Default)]
pub(crate) struct CodeBlockInfo {
    pub code_key: CodeKey,
    pub head_line_no: usize,
    pub row_count: usize,
    pub lang: Option<String>,
    pub plantuml: PlantumlRenderState,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PlantumlRenderState {
    pub code_digest: Option<u64>,
    image: Option<CachedPlantumlImage>,
}

impl PlantumlRenderState {
    fn set_image_url(&mut self, image_url: Option<String>) {
        self.image = image_url.map(CachedPlantumlImage::new);
    }

    pub(crate) fn image_url(&self) -> Option<String> {
        self.image.as_ref().map(|p| p.url().to_string())
    }
}

#[derive(Clone, Debug)]
struct CachedPlantumlImage {
    url: Arc<String>,
}

impl CachedPlantumlImage {
    fn new(url: String) -> Self {
        Self { url: Arc::new(url) }
    }

    fn url(&self) -> &str {
        self.url.as_str()
    }

    fn remove_cache_file(url: &str) {
        let Some(path_text) = url.strip_prefix("file://") else {
            return;
        };
        let path = PathBuf::from(path_text);
        if let Err(e) = fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::debug!("plantuml remove stale cache file failed: {}", e);
            }
        }
    }
}

impl Drop for CachedPlantumlImage {
    fn drop(&mut self) {
        if Arc::strong_count(&self.url) == 1 {
            Self::remove_cache_file(self.url());
        }
    }
}

/// fenced 代码块缓存：`CodeKey` → [`CodeBlockInfo`]。
#[derive(Debug)]
pub(crate) struct CodeCache {
    store: HashMap<CodeKey, CodeBlockInfo>,
    next_code_key: CodeKey,
    used_code_keys: HashSet<CodeKey>,
    pending_renders: HashMap<CodeKey, PendingRender>,
}

#[derive(Debug)]
struct PendingRender {
    digest: u64,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Option<String>>,
}

impl Default for CodeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeCache {
    const PLANTUML_LANG: &'static str = "plantuml";
    const PLANTUML_JAR_FILE: &'static str = "plantuml-1.2025.2.jar";

    pub(crate) fn new() -> Self {
        Self {
            store: HashMap::new(),
            next_code_key: 1,
            used_code_keys: HashSet::new(),
            pending_renders: HashMap::new(),
        }
    }

    pub(crate) fn alloc_code_key(&mut self) -> CodeKey {
        let key = self.next_code_key;
        self.next_code_key = self.next_code_key.saturating_add(1);
        key
    }

    pub(crate) fn code_info_by_key(&self, code_key: CodeKey) -> Option<&CodeBlockInfo> {
        self.store.get(&code_key)
    }

    pub(crate) fn code_info_cloned_by_key(&self, code_key: CodeKey) -> Option<CodeBlockInfo> {
        self.store.get(&code_key).cloned()
    }

    fn upsert_code_info(&mut self, info: CodeBlockInfo) {
        log::trace!(
            "code_cache upsert key={} head={} rows={} lang={:?}",
            info.code_key,
            info.head_line_no,
            info.row_count,
            info.lang
        );
        self.store.insert(info.code_key, info);
    }

    pub(crate) fn upsert_code_info_by_key(&mut self, code_key: CodeKey, mut info: CodeBlockInfo) {
        info.code_key = code_key;
        self.upsert_code_info(info);
    }

    fn mark_used(&mut self, code_key: CodeKey) {
        self.used_code_keys.insert(code_key);
    }

    fn retain_used_only(&mut self) {
        self.store.retain(|k, _| self.used_code_keys.contains(k));
        let keys: Vec<CodeKey> = self.pending_renders.keys().copied().collect();
        for key in keys {
            if !self.used_code_keys.contains(&key) {
                self.cancel_pending_render(key);
            }
        }
    }

    fn cancel_pending_render(&mut self, code_key: CodeKey) {
        if let Some(pending) = self.pending_renders.remove(&code_key) {
            pending.cancel.store(true, Ordering::Relaxed);
        }
    }

    fn calc_digest(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn code_block_text(ctx: &Ctx, s: usize, e: usize) -> String {
        let mut body = String::new();
        for ln in s..=e {
            if ln > s {
                body.push('\n');
            }
            if let Some(p) = ctx.get_line(ln) {
                body.push_str(&p.get_text());
            }
        }
        body
    }

    fn resolve_plantuml_jar_path(cfg_path: Option<&str>) -> Option<PathBuf> {
        if let Some(p) = cfg_path {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
        if let Ok(cur_dir) = std::env::current_dir() {
            let path = cur_dir.join(Self::PLANTUML_JAR_FILE);
            if path.exists() {
                return Some(path);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let path = parent.join(Self::PLANTUML_JAR_FILE);
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    fn ensure_plantuml_wrapped(code: &str) -> String {
        let trimmed = code.trim();
        if trimmed.starts_with("@startuml") && trimmed.ends_with("@enduml") {
            code.to_string()
        } else {
            format!("@startuml\n{}\n@enduml\n", code)
        }
    }

    fn render_plantuml_png(
        code_key: CodeKey,
        digest: u64,
        code_text: &str,
        jar_path: &Path,
        cancel: &Arc<AtomicBool>,
    ) -> Option<String> {
        fn cleanup_temp_puml(path: &Path) {
            if let Err(e) = fs::remove_file(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::debug!("plantuml remove temp puml failed: {}", e);
                }
            }
        }

        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let base_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
            .filter(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("."));
        let cache_dir = base_dir.join("cache").join("plantuml");
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            log::warn!("plantuml create cache dir failed: {}", e);
            return None;
        }

        let stem = format!("code_{}_{}", code_key, digest);
        let puml_path = cache_dir.join(format!("{}.puml", stem));
        let wrapped = Self::ensure_plantuml_wrapped(code_text);
        if let Err(e) = fs::write(&puml_path, wrapped) {
            log::warn!("plantuml write puml failed: {}", e);
            return None;
        }
        if cancel.load(Ordering::Relaxed) {
            cleanup_temp_puml(&puml_path);
            return None;
        }

        let mut cmd = Command::new("java");
        cmd.arg("-jar")
            .arg(jar_path)
            .arg("-tpng")
            .arg("-charset")
            .arg("UTF-8")
            .arg("-o")
            .arg(&cache_dir)
            .arg(&puml_path);
        #[cfg(windows)]
        {
            cmd.creation_flags(crate::util::win_exec::CREATE_NO_WINDOW);
        }
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                log::warn!("plantuml render command failed: {}", e);
                cleanup_temp_puml(&puml_path);
                return None;
            }
        };
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                cleanup_temp_puml(&puml_path);
                return None;
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        log::warn!("plantuml render failed with status: {}", status);
                        cleanup_temp_puml(&puml_path);
                        return None;
                    }
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(30));
                }
                Err(e) => {
                    log::warn!("plantuml render command wait failed: {}", e);
                    cleanup_temp_puml(&puml_path);
                    return None;
                }
            }
        }
        let png_path = cache_dir.join(format!("{}.png", stem));
        if !png_path.exists() {
            log::warn!("plantuml render success but png not found: {}", png_path.display());
            cleanup_temp_puml(&puml_path);
            return None;
        }
        cleanup_temp_puml(&puml_path);
        Some(format!("file://{}", png_path.to_string_lossy().replace('\\', "/")))
    }

    fn spawn_render_plantuml_png(
        code_key: CodeKey,
        digest: u64,
        code_text: String,
        jar_path: PathBuf,
    ) -> PendingRender {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_worker = Arc::clone(&cancel);
        log::info!(
            "plantuml spawn render code_key={} digest={} jar={}",
            code_key,
            digest,
            jar_path.display()
        );
        thread::spawn(move || {
            let rendered =
                Self::render_plantuml_png(code_key, digest, &code_text, &jar_path, &cancel_for_worker);
            let _ = tx.send(rendered);
        });
        PendingRender { digest, cancel, rx }
    }

    pub(crate) fn poll_render_results(&mut self) -> Vec<usize> {
        let mut flash_lines = Vec::new();
        let keys: Vec<CodeKey> = self.pending_renders.keys().copied().collect();
        for key in keys {
            let Some(pending) = self.pending_renders.get(&key) else {
                continue;
            };
            match pending.rx.try_recv() {
                Ok(image_url) => {
                    if let Some(info) = self.store.get_mut(&key) {
                        if info.plantuml.code_digest == Some(pending.digest) {
                            info.plantuml.set_image_url(image_url);
                            let tail_line = info.head_line_no + info.row_count.saturating_sub(1);
                            flash_lines.push(tail_line);
                            log::info!(
                                "plantuml render ready code_key={} digest={} tail_line={} has_image={}",
                                key,
                                pending.digest,
                                tail_line,
                                info.plantuml.image.is_some()
                            );
                        } else {
                            log::info!(
                                "plantuml render stale code_key={} digest={} current_digest={:?}",
                                key,
                                pending.digest,
                                info.plantuml.code_digest
                            );
                        }
                    }
                    self.cancel_pending_render(key);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    log::warn!("plantuml render channel disconnected code_key={}", key);
                    self.cancel_pending_render(key);
                }
            }
        }
        flash_lines
    }
}

impl IndexCache for CodeCache {
    fn rebuild_index_init(&mut self, _gen: u64) {
        self.used_code_keys.clear();
        let _ = self.poll_render_results();
    }

    fn rebuild_index_step(&mut self, ctx: &mut Ctx, line_no: usize) -> usize {
        for tail_line in self.poll_render_results() {
            ctx.line_flash_tick(tail_line);
        }
        if line_no >= ctx.line_num() {
            return ctx.line_num();
        }
        if ctx
            .get_line(line_no)
            .map(|p| p.is_code_row())
            .unwrap_or(false)
        {
            if let Some((s, e)) = ctx.code_row_block_range(line_no) {
                let n = e.saturating_sub(s) + 1;
                let mut code_key = ctx.get_line(s).and_then(|p| p.code_key).unwrap_or(0);
                if code_key == 0 {
                    code_key = self.alloc_code_key();
                }
                let mut base = self.code_info_cloned_by_key(code_key).unwrap_or_default();
                base.code_key = code_key;
                base.head_line_no = s;
                base.row_count = n;
                base.lang = ctx.get_line(s).and_then(|p| p.code_lang.clone());
                let lang_lc = base.lang.clone().unwrap_or_default().to_ascii_lowercase();
                if lang_lc == Self::PLANTUML_LANG {
                    let code_text = Self::code_block_text(ctx, s, e);
                    let digest = Self::calc_digest(&code_text);
                    if base.plantuml.code_digest != Some(digest) {
                        base.plantuml.code_digest = Some(digest);
                        base.plantuml.set_image_url(None);
                        self.cancel_pending_render(code_key);
                        let jar_path = Self::resolve_plantuml_jar_path(ctx.cfg().plantuml_jar_path.as_deref());
                        if let Some(jar_path) = jar_path {
                            let pending = Self::spawn_render_plantuml_png(
                                code_key,
                                digest,
                                code_text,
                                jar_path,
                            );
                            self.pending_renders.insert(code_key, pending);
                        }
                    }
                } else {
                    base.plantuml.code_digest = None;
                    base.plantuml.set_image_url(None);
                    self.cancel_pending_render(code_key);
                }
                let top = ctx.cfg().spacing.code.top;
                let bottom = ctx.cfg().spacing.code.bottom;

                self.upsert_code_info(base.clone());
                self.mark_used(code_key);

                for (i, ln) in (s..=e).enumerate() {
                    if let Some(p) = ctx.get_line_mut(ln) {
                        if !p.is_code_row() {
                            continue;
                        }
                        p.code_key = Some(code_key);
                        if ln == s {
                            p.code_lang = base.lang.clone();
                        } else {
                            p.code_lang = None;
                        }
                        p.spacing_top = if i == 0 { top } else { 0.0 };
                        p.spacing_bottom = if i + 1 == n { bottom } else { 0.0 };
                    }
                }
                return e.saturating_add(1);
            }
        }
        if let Some(p) = ctx.get_line_mut(line_no) {
            p.code_key = None;
        }
        line_no.saturating_add(1)
    }

    fn rebuild_index_end(&mut self, _ctx: &mut Ctx, _gen: u64) {
        self.retain_used_only();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
