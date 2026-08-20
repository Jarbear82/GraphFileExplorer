use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use anyhow::{Context as AnyhowContext, Result};
use gpui::{Context, Entity, Window};
use gpui_component::dock::{DockArea, DockSkin};

use crate::model::fs_entry::FsEntry;
use crate::model::layout::{
    LayoutKind, LayoutOptions, LayoutResult, create_layout_engine,
};

pub struct Workspace {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    pub selected_path: Option<PathBuf>,
    pub current_entry: Option<FsEntry>,
    pub layout_kind: LayoutKind,
    pub layout_result: Option<LayoutResult>,
    pub layout_options: LayoutOptions,
    pub filter_query: String,
    pub show_hidden: bool,
    pub expanded_paths: std::collections::HashSet<PathBuf>,
    pub recent_paths: Vec<PathBuf>,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
    pub dock_area: Entity<DockArea>,
    pub dock_skin: Rc<DockSkin>,
}

impl Workspace {
    pub fn new(initial_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (dock_area, dock_skin) = DockSkin::dock_area("main-dock", Some(1), window, cx);

        let root_path = initial_path
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let mut workspace = Self {
            root_path: root_path.clone(),
            current_path: root_path.clone(),
            selected_path: None,
            current_entry: None,
            layout_kind: LayoutKind::NativeTopDown,
            layout_result: None,
            layout_options: LayoutOptions::default(),
            filter_query: String::new(),
            show_hidden: false,
            expanded_paths: std::collections::HashSet::new(),
            recent_paths: vec![root_path.clone()],
            is_loading: false,
            status_message: Some(format!("Opened {}", root_path.display())),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            dock_area,
            dock_skin,
        };

        workspace.load_current_directory(cx);
        workspace
    }

    pub fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        let path = self.current_path.clone();
        let show_hidden = self.show_hidden;
        let filter = self.filter_query.to_lowercase();

        let mut root_entry = FsEntry::from_path(&path, false, 0, show_hidden);
        root_entry.load_children_with_expanded(&self.expanded_paths, 1, show_hidden);

        if !filter.is_empty() {
            root_entry.children.retain(|c| c.name.to_lowercase().contains(&filter));
        }

        self.layout_options.expanded_paths = self.expanded_paths.clone();
        let engine = create_layout_engine(self.layout_kind);
        let layout = engine.compute_layout(&root_entry, 1000.0, 700.0, &self.layout_options);

        self.current_entry = Some(root_entry);
        self.layout_result = Some(layout);
        self.is_loading = false;

        cx.notify();
    }

    pub fn toggle_expand(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.expanded_paths.contains(&path) {
            self.expanded_paths.remove(&path);
        } else {
            self.expanded_paths.insert(path);
        }
        self.load_current_directory(cx);
    }

    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded_paths.contains(path)
    }

    pub fn can_go_back(&self) -> bool {
        !self.history_back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history_forward.is_empty()
    }

    pub fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.current_path.clone());
            self.current_path = prev;
            self.selected_path = None;
            self.status_message = Some(format!("Navigated back to {}", self.current_path.display()));
            self.load_current_directory(cx);
        }
    }

    pub fn navigate_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.current_path.clone());
            self.current_path = next;
            self.selected_path = None;
            self.status_message = Some(format!("Navigated forward to {}", self.current_path.display()));
            self.load_current_directory(cx);
        }
    }

    pub fn open_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.is_dir() {
            if path != self.current_path {
                self.history_back.push(self.current_path.clone());
                self.history_forward.clear();
            }
            self.root_path = path.clone();
            self.current_path = path.clone();
            self.selected_path = None;
            if !self.recent_paths.contains(&path) {
                self.recent_paths.insert(0, path.clone());
            }
            self.status_message = Some(format!("Opened root: {}", path.display()));
            self.load_current_directory(cx);
        }
    }

    pub fn drill_down(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            return;
        }

        if path != self.current_path {
            self.history_back.push(self.current_path.clone());
            self.history_forward.clear();
        }

        self.current_path = path;
        self.selected_path = None;
        self.filter_query.clear();
        self.status_message = Some(format!("Navigated to {}", self.current_path.display()));
        self.load_current_directory(cx);
    }

    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.current_path.parent().map(|p| p.to_path_buf()) {
            if parent != self.current_path {
                self.history_back.push(self.current_path.clone());
                self.history_forward.clear();
                self.current_path = parent;
                self.selected_path = None;
                self.filter_query.clear();
                self.status_message = Some(format!("Navigated up to {}", self.current_path.display()));
                self.load_current_directory(cx);
            }
        }
    }

    pub fn select_path(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.selected_path = path;
        cx.notify();
    }

    pub fn set_layout_kind(&mut self, kind: LayoutKind, cx: &mut Context<Self>) {
        self.layout_kind = kind;
        self.status_message = Some(format!("Switched layout to {}", kind.name()));
        self.load_current_directory(cx);
    }

    pub fn set_filter_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.filter_query = query;
        self.load_current_directory(cx);
    }

    pub fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        self.show_hidden = !self.show_hidden;
        self.status_message = Some(if self.show_hidden {
            "Showing hidden files".to_string()
        } else {
            "Hiding hidden files".to_string()
        });
        self.load_current_directory(cx);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load_current_directory(cx);
    }

    pub fn create_entry(&mut self, name: &str, is_dir: bool, cx: &mut Context<Self>) -> Result<()> {
        let target = self.current_path.join(name);
        if target.exists() {
            anyhow::bail!("Item already exists");
        }

        if is_dir {
            fs::create_dir_all(&target).context("Failed to create directory")?;
            self.status_message = Some(format!("Created directory: {name}"));
        } else {
            fs::write(&target, "").context("Failed to create file")?;
            self.status_message = Some(format!("Created file: {name}"));
        }

        self.selected_path = Some(target);
        self.load_current_directory(cx);
        Ok(())
    }

    pub fn rename_entry(&mut self, old_path: &Path, new_name: &str, cx: &mut Context<Self>) -> Result<()> {
        let parent = old_path.parent().context("Invalid parent")?;
        let new_path = parent.join(new_name);

        if new_path.exists() {
            anyhow::bail!("Destination name already exists");
        }

        fs::rename(old_path, &new_path).context("Failed to rename")?;
        self.status_message = Some(format!("Renamed to {new_name}"));
        self.selected_path = Some(new_path);
        self.load_current_directory(cx);
        Ok(())
    }

    pub fn delete_entry(&mut self, path: &Path, cx: &mut Context<Self>) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path).context("Failed to delete directory")?;
        } else {
            fs::remove_file(path).context("Failed to delete file")?;
        }

        self.status_message = Some(format!("Deleted {}", path.display()));
        if self.selected_path.as_deref() == Some(path) {
            self.selected_path = None;
        }

        self.load_current_directory(cx);
        Ok(())
    }

    pub fn open_in_system_editor(path: &Path) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd").args(["/C", "start", "", &path.to_string_lossy()]).spawn();
        }
    }

    pub fn reveal_in_file_manager(path: &Path) {
        let parent = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(parent).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(parent).spawn();
        }
    }
}
