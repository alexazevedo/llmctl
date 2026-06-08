use llmctl::{
    ModelInfo, ModelStatus, ProcessInfo, default_home, detect_llamacpp_loaded, format_bytes,
    ollama_models_from_manifests, parse_ollama_list, parse_ollama_ps, relevant_processes,
    remove_command, scan_llamacpp_files, scan_lmstudio_models,
};
use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "list".to_string());
    let result = match command.as_str() {
        "list" => print_list(),
        "running" | "loaded" => print_running(),
        "storage" => print_storage(),
        "remove" => {
            let provider = args.next();
            let model = args.next();
            print_remove(provider, model)
        }
        "doctor" => print_doctor(),
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!("llmctl - local LLM inventory");
    println!();
    println!("Usage:");
    println!("  llmctl [list]");
    println!("  llmctl running");
    println!("  llmctl storage");
    println!("  llmctl remove <provider> <model>");
    println!("  llmctl doctor");
    println!();
    println!("Optional:");
    println!("  LLMCTL_GGUF_DIRS=/path/one:/path/two llmctl list");
}

fn print_list() -> Result<(), String> {
    let mut models = installed_models()?;
    let running = running_models()?;
    for loaded in running {
        if let Some(model) = models
            .iter_mut()
            .find(|model| model.provider == loaded.provider && model.name == loaded.name)
        {
            model.status = ModelStatus::Loaded;
            model.pid = loaded.pid;
            model.cpu_percent = loaded.cpu_percent;
            model.memory_bytes = loaded.memory_bytes;
        } else {
            models.push(loaded);
        }
    }
    print_models(&models);
    Ok(())
}

fn print_running() -> Result<(), String> {
    let mut rows = running_models()?;
    let processes = relevant_processes().unwrap_or_default();
    rows.extend(processes_to_rows(processes));
    print_models(&rows);
    Ok(())
}

fn print_storage() -> Result<(), String> {
    let models = installed_models()?;
    print_models(&models);
    let total: u64 = models.iter().filter_map(|model| model.size_bytes).sum();
    println!();
    println!("Total detected storage: {}", format_bytes(total));
    Ok(())
}

fn print_remove(provider: Option<String>, model: Option<String>) -> Result<(), String> {
    let provider = provider.ok_or_else(|| "missing provider".to_string())?;
    let model = model.ok_or_else(|| "missing model".to_string())?;
    println!("{}", remove_command(&provider, &model)?);
    Ok(())
}

fn print_doctor() -> Result<(), String> {
    println!("ollama: {}", command_status("ollama"));
    println!("lms: {}", command_status("lms"));
    println!("ps: {}", command_status("ps"));
    let home = default_home();
    println!("ollama models: {}", home.join(".ollama/models").display());
    println!(
        "lmstudio models: {}",
        home.join(".lmstudio/models").display()
    );
    println!(
        "llama.cpp scan roots: {}",
        std::env::var_os("LLMCTL_GGUF_DIRS")
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "not configured".to_string())
    );
    Ok(())
}

fn installed_models() -> Result<Vec<ModelInfo>, String> {
    let home = default_home();
    let mut models = Vec::new();
    models.extend(ollama_installed(&home));
    models.extend(
        scan_lmstudio_models(&home.join(".lmstudio/models")).map_err(|error| error.to_string())?,
    );
    models
        .extend(scan_llamacpp_files(&configured_gguf_roots()).map_err(|error| error.to_string())?);
    models.sort_by(|a, b| (&a.provider, &a.name).cmp(&(&b.provider, &b.name)));
    Ok(models)
}

fn running_models() -> Result<Vec<ModelInfo>, String> {
    let mut models = ollama_loaded();
    let processes = relevant_processes().unwrap_or_default();
    models.extend(detect_llamacpp_loaded(&processes));
    Ok(models)
}

fn configured_gguf_roots() -> Vec<std::path::PathBuf> {
    std::env::var_os("LLMCTL_GGUF_DIRS")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default()
}

fn ollama_installed(home: &std::path::Path) -> Vec<ModelInfo> {
    if let Ok(output) = Command::new("ollama").arg("list").output() {
        if output.status.success() {
            let models = parse_ollama_list(&String::from_utf8_lossy(&output.stdout));
            if !models.is_empty() {
                return models;
            }
        }
    }
    ollama_models_from_manifests(&home.join(".ollama/models/manifests")).unwrap_or_default()
}

fn ollama_loaded() -> Vec<ModelInfo> {
    if let Ok(output) = Command::new("ollama").arg("ps").output() {
        if output.status.success() {
            return parse_ollama_ps(&String::from_utf8_lossy(&output.stdout));
        }
    }
    Vec::new()
}

fn processes_to_rows(processes: Vec<ProcessInfo>) -> Vec<ModelInfo> {
    processes
        .into_iter()
        .map(|process| ModelInfo {
            provider: process_provider(&process.command).to_string(),
            name: process.command,
            status: ModelStatus::Process,
            size_bytes: None,
            size_text: None,
            pid: Some(process.pid),
            cpu_percent: process.cpu_percent,
            memory_bytes: process.memory_bytes,
            path: None,
        })
        .collect()
}

fn process_provider(command: &str) -> &'static str {
    let lower = command.to_lowercase();
    if lower.contains("ollama") {
        "ollama"
    } else if lower.contains("lm studio") || lower.contains("lmstudio") || lower.contains("lms") {
        "lmstudio"
    } else {
        "llama.cpp"
    }
}

fn print_models(models: &[ModelInfo]) {
    println!(
        "{:<12} {:<60} {:<10} {:>10} {:>8} {:>10} {:>8}",
        "Provider", "Model", "Status", "Size", "CPU", "Memory", "PID"
    );
    for model in models {
        println!(
            "{:<12} {:<60} {:<10} {:>10} {:>8} {:>10} {:>8}",
            model.provider,
            truncate(&model.name, 60),
            model.status,
            model.size_label(),
            model.cpu_label(),
            model.memory_label(),
            model
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut output: String = value.chars().take(width.saturating_sub(3)).collect();
    output.push_str("...");
    output
}

fn command_status(command: &str) -> &'static str {
    if Command::new(command).arg("--help").output().is_ok() {
        "found"
    } else {
        "missing"
    }
}
