use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    Directory,
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    C,
    Cpp,
    Html,
    Css,
    Json,
    Toml,
    Yaml,
    Markdown,
    Image,
    Audio,
    Video,
    Archive,
    Executable,
    Document,
    Other,
}

impl FileCategory {
    pub fn from_extension(ext: Option<&str>, is_dir: bool) -> Self {
        if is_dir {
            return FileCategory::Directory;
        }

        match ext.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("rs") => FileCategory::Rust,
            Some("js") | Some("mjs") | Some("cjs") | Some("jsx") => FileCategory::JavaScript,
            Some("ts") | Some("mts") | Some("cts") | Some("tsx") => FileCategory::TypeScript,
            Some("py") | Some("pyw") | Some("ipynb") => FileCategory::Python,
            Some("go") => FileCategory::Go,
            Some("c") | Some("h") => FileCategory::C,
            Some("cpp") | Some("cxx") | Some("cc") | Some("hpp") | Some("hxx") => FileCategory::Cpp,
            Some("html") | Some("htm") => FileCategory::Html,
            Some("css") | Some("scss") | Some("sass") | Some("less") => FileCategory::Css,
            Some("json") => FileCategory::Json,
            Some("toml") => FileCategory::Toml,
            Some("yaml") | Some("yml") => FileCategory::Yaml,
            Some("md") | Some("markdown") | Some("mdx") => FileCategory::Markdown,
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") | Some("ico") => {
                FileCategory::Image
            }
            Some("mp3") | Some("wav") | Some("ogg") | Some("flac") | Some("m4a") => FileCategory::Audio,
            Some("mp4") | Some("mkv") | Some("webm") | Some("mov") | Some("avi") => FileCategory::Video,
            Some("zip") | Some("tar") | Some("gz") | Some("7z") | Some("rar") | Some("bz2") | Some("xz") => {
                FileCategory::Archive
            }
            Some("exe") | Some("bin") | Some("sh") | Some("bash") | Some("bat") | Some("cmd") => {
                FileCategory::Executable
            }
            Some("pdf") | Some("doc") | Some("docx") | Some("txt") | Some("rtf") => FileCategory::Document,
            _ => FileCategory::Other,
        }
    }

    pub fn display_badge(&self) -> &'static str {
        match self {
            FileCategory::Directory => "DIR",
            FileCategory::Rust => "RS",
            FileCategory::JavaScript => "JS",
            FileCategory::TypeScript => "TS",
            FileCategory::Python => "PY",
            FileCategory::Go => "GO",
            FileCategory::C => "C",
            FileCategory::Cpp => "C++",
            FileCategory::Html => "HTML",
            FileCategory::Css => "CSS",
            FileCategory::Json => "JSON",
            FileCategory::Toml => "TOML",
            FileCategory::Yaml => "YAML",
            FileCategory::Markdown => "MD",
            FileCategory::Image => "IMG",
            FileCategory::Audio => "AUD",
            FileCategory::Video => "VID",
            FileCategory::Archive => "ZIP",
            FileCategory::Executable => "EXE",
            FileCategory::Document => "DOC",
            FileCategory::Other => "FILE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
    pub extension: Option<String>,
    pub category: FileCategory,
    pub children: Vec<FsEntry>,
    pub is_loaded: bool,
    pub item_count: usize,
}

impl FsEntry {
    pub fn from_path(path: &Path, load_children: bool, depth_limit: usize, show_hidden: bool) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let metadata = fs::symlink_metadata(path).ok();
        let is_symlink = metadata.as_ref().map(|m| m.file_type().is_symlink()).unwrap_or(false);
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(path.is_dir());
        let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata.and_then(|m| m.modified().ok());

        let extension = if is_dir {
            None
        } else {
            path.extension().map(|e| e.to_string_lossy().to_string())
        };

        let category = FileCategory::from_extension(extension.as_deref(), is_dir);

        let mut entry = Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            is_symlink,
            size_bytes,
            modified,
            extension,
            category,
            children: Vec::new(),
            is_loaded: false,
            item_count: 0,
        };

        if is_dir && load_children {
            entry.load_children(depth_limit, show_hidden);
        }

        entry
    }

    pub fn load_children(&mut self, depth_limit: usize, show_hidden: bool) {
        let empty_set = std::collections::HashSet::new();
        self.load_children_with_expanded(&empty_set, depth_limit, show_hidden);
    }

    pub fn load_children_with_expanded(
        &mut self,
        expanded_paths: &std::collections::HashSet<PathBuf>,
        base_preview_depth: usize,
        show_hidden: bool,
    ) {
        if !self.is_dir {
            return;
        }

        let mut children = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&self.path) {
            for dir_entry in read_dir.flatten() {
                let child_path = dir_entry.path();
                let file_name = child_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if !show_hidden && file_name.starts_with('.') {
                    continue;
                }

                let is_child_expanded = expanded_paths.contains(&child_path);
                let next_preview_depth = if is_child_expanded {
                    base_preview_depth.max(1)
                } else if base_preview_depth > 0 {
                    base_preview_depth - 1
                } else {
                    0
                };

                let should_load = is_child_expanded || base_preview_depth > 0;
                let mut child = FsEntry::from_path(&child_path, false, 0, show_hidden);
                if child.is_dir && should_load {
                    child.load_children_with_expanded(expanded_paths, next_preview_depth, show_hidden);
                }
                children.push(child);
            }
        }

        // Sort: directories first, then alphabetical case-insensitive
        children.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        self.item_count = children.len();
        self.children = children;
        self.is_loaded = true;
    }

    pub fn format_size(&self) -> String {
        if self.is_dir {
            format!("{} items", self.item_count)
        } else {
            format_bytes(self.size_bytes)
        }
    }

    pub fn format_modified(&self) -> String {
        match self.modified {
            Some(time) => {
                if let Ok(elapsed) = time.elapsed() {
                    let secs = elapsed.as_secs();
                    if secs < 60 {
                        "Just now".to_string()
                    } else if secs < 3600 {
                        format!("{}m ago", secs / 60)
                    } else if secs < 86400 {
                        format!("{}h ago", secs / 3600)
                    } else {
                        format!("{}d ago", secs / 86400)
                    }
                } else {
                    "In the future".to_string()
                }
            }
            None => "Unknown".to_string(),
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
}
