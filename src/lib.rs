use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelStatus {
    Installed,
    Loaded,
    Process,
}

impl fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelStatus::Installed => write!(f, "installed"),
            ModelStatus::Loaded => write!(f, "loaded"),
            ModelStatus::Process => write!(f, "process"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub provider: String,
    pub name: String,
    pub status: ModelStatus,
    pub size_bytes: Option<u64>,
    pub size_text: Option<String>,
    pub pid: Option<u32>,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub path: Option<PathBuf>,
}

impl ModelInfo {
    pub fn size_label(&self) -> String {
        match (&self.size_text, self.size_bytes) {
            (Some(size), _) => size.clone(),
            (None, Some(bytes)) => format_bytes(bytes),
            (None, None) => "-".to_string(),
        }
    }

    pub fn memory_label(&self) -> String {
        self.memory_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn cpu_label(&self) -> String {
        self.cpu_percent
            .map(|cpu| format!("{cpu:.1}%"))
            .unwrap_or_else(|| "-".to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub command: String,
}

pub fn default_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn parse_ollama_list(output: &str) -> Vec<ModelInfo> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 4 {
                return None;
            }
            Some(ModelInfo {
                provider: "ollama".to_string(),
                name: columns[0].to_string(),
                status: ModelStatus::Installed,
                size_bytes: None,
                size_text: Some(format!("{} {}", columns[2], columns[3])),
                pid: None,
                cpu_percent: None,
                memory_bytes: None,
                path: None,
            })
        })
        .collect()
}

pub fn parse_ollama_ps(output: &str) -> Vec<ModelInfo> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 4 {
                return None;
            }
            Some(ModelInfo {
                provider: "ollama".to_string(),
                name: columns[0].to_string(),
                status: ModelStatus::Loaded,
                size_bytes: None,
                size_text: Some(format!("{} {}", columns[2], columns[3])),
                pid: None,
                cpu_percent: None,
                memory_bytes: None,
                path: None,
            })
        })
        .collect()
}

pub fn ollama_models_from_manifests(root: &Path) -> io::Result<Vec<ModelInfo>> {
    let mut files = Vec::new();
    collect_files(root, &mut files)?;
    let mut models = Vec::new();
    for file in files {
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let parts: Vec<String> = relative
            .iter()
            .map(|part| part.to_string_lossy().to_string())
            .collect();
        if parts.len() < 4 {
            continue;
        }
        let tag = parts.last().cloned().unwrap_or_default();
        let model = parts[parts.len() - 2].clone();
        let namespace = parts[..parts.len() - 2].join("/");
        let name = if namespace == "registry.ollama.ai/library" {
            format!("{model}:{tag}")
        } else {
            format!("{namespace}/{model}:{tag}")
        };
        models.push(ModelInfo {
            provider: "ollama".to_string(),
            name,
            status: ModelStatus::Installed,
            size_bytes: None,
            size_text: None,
            pid: None,
            cpu_percent: None,
            memory_bytes: None,
            path: Some(file),
        });
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

pub fn scan_lmstudio_models(root: &Path) -> io::Result<Vec<ModelInfo>> {
    let mut models = Vec::new();
    if !root.exists() {
        return Ok(models);
    }
    for org in fs::read_dir(root)? {
        let org = org?;
        if !org.file_type()?.is_dir() {
            continue;
        }
        for model in fs::read_dir(org.path())? {
            let model = model?;
            if !model.file_type()?.is_dir() {
                continue;
            }
            let path = model.path();
            let name = format!(
                "{}/{}",
                org.file_name().to_string_lossy(),
                model.file_name().to_string_lossy()
            );
            models.push(ModelInfo {
                provider: "lmstudio".to_string(),
                name,
                status: ModelStatus::Installed,
                size_bytes: Some(dir_size(&path)?),
                size_text: None,
                pid: None,
                cpu_percent: None,
                memory_bytes: None,
                path: Some(path),
            });
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

pub fn detect_llamacpp_loaded(processes: &[ProcessInfo]) -> Vec<ModelInfo> {
    processes
        .iter()
        .filter(|process| is_llamacpp_process(&process.command))
        .filter_map(|process| {
            let model_path = extract_gguf_path(&process.command)?;
            Some(ModelInfo {
                provider: "llama.cpp".to_string(),
                name: model_path.clone(),
                status: ModelStatus::Loaded,
                size_bytes: fs::metadata(&model_path).map(|m| m.len()).ok(),
                size_text: None,
                pid: Some(process.pid),
                cpu_percent: process.cpu_percent,
                memory_bytes: process.memory_bytes,
                path: Some(PathBuf::from(model_path)),
            })
        })
        .collect()
}

pub fn scan_llamacpp_files(roots: &[PathBuf]) -> io::Result<Vec<ModelInfo>> {
    let mut models = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        let mut files = Vec::new();
        collect_files(root, &mut files)?;
        for file in files {
            if file.extension() != Some(OsStr::new("gguf")) {
                continue;
            }
            models.push(ModelInfo {
                provider: "llama.cpp".to_string(),
                name: file.to_string_lossy().to_string(),
                status: ModelStatus::Installed,
                size_bytes: fs::metadata(&file).map(|m| m.len()).ok(),
                size_text: None,
                pid: None,
                cpu_percent: None,
                memory_bytes: None,
                path: Some(file),
            });
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

pub fn relevant_processes() -> io::Result<Vec<ProcessInfo>> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,pcpu=,rss=,command="])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ps_output(&stdout)
        .into_iter()
        .filter(|process| {
            let command = process.command.to_lowercase();
            command.contains("ollama")
                || command.contains("lm studio")
                || command.contains("lmstudio")
                || command.contains("lms")
                || is_llamacpp_process(&command)
        })
        .collect())
}

pub fn parse_ps_output(output: &str) -> Vec<ProcessInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse().ok()?;
            let cpu_percent = parts.next().and_then(|value| value.parse().ok());
            let rss_kb: Option<u64> = parts.next().and_then(|value| value.parse().ok());
            let command = parts.collect::<Vec<&str>>().join(" ");
            if command.is_empty() {
                return None;
            }
            Some(ProcessInfo {
                pid,
                cpu_percent,
                memory_bytes: rss_kb.map(|kb| kb * 1024),
                command,
            })
        })
        .collect()
}

pub fn remove_command(provider: &str, model: &str) -> Result<String, String> {
    match provider {
        "ollama" => Ok(format!("ollama rm {model}")),
        "lmstudio" => Ok(format!(
            "lms unload {model} && rm -rf ~/.lmstudio/models/{model}"
        )),
        "llama.cpp" | "llamacpp" => Ok(format!("rm {model}")),
        other => Err(format!("unknown provider: {other}")),
    }
}

pub fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = units[0];
    for next in units.iter().skip(1) {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    if unit == "B" {
        format!("{bytes} B")
    } else if value >= 10.0 {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.1} {unit}")
    }
}

fn is_llamacpp_process(command: &str) -> bool {
    let command = command.to_lowercase();
    command.contains("llama.cpp")
        || command.contains("llama-server")
        || command.contains("llama-cli")
        || command.contains("/main ")
        || command.ends_with("/main")
}

fn extract_gguf_path(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .find(|part| part.ends_with(".gguf"))
        .map(|part| part.trim_matches('"').trim_matches('\'').to_string())
}

fn dir_size(path: &Path) -> io::Result<u64> {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                stack.push(entry.path());
            } else if metadata.is_file() {
                total += metadata.len();
            }
        }
    }
    Ok(total)
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_files(&entry.path(), files)?;
        } else if metadata.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}
