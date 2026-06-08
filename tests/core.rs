use llmctl::{
    ModelStatus, ProcessInfo, detect_llamacpp_loaded, parse_ollama_list, remove_command,
    scan_lmstudio_models,
};
use std::fs;

#[test]
fn parses_ollama_list_output() {
    let output = "\
NAME                                      ID              SIZE       MODIFIED
hf.co/unsloth/gemma-4-12b-it-GGUF:BF16    62bdaffafe8d    1.0 GB     17 minutes ago
gemma4:12b-mlx                            7c75d6f0f4b9    10.0 GB    50 minutes ago
qwen3.6:27b                               a50eda8ed977    17 GB      4 weeks ago
";

    let models = parse_ollama_list(output);

    assert_eq!(models.len(), 3);
    assert_eq!(models[0].provider, "ollama");
    assert_eq!(models[0].name, "hf.co/unsloth/gemma-4-12b-it-GGUF:BF16");
    assert_eq!(models[0].size_label(), "1.0 GB");
    assert_eq!(models[2].name, "qwen3.6:27b");
    assert_eq!(models[2].size_label(), "17 GB");
}

#[test]
fn scans_lmstudio_org_model_directories_and_sizes_them() {
    let root = std::env::temp_dir().join(format!("llmctl-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let model_dir = root
        .join("lmstudio-community")
        .join("Qwen3-Coder-Next-MLX-4bit");
    fs::create_dir_all(&model_dir).unwrap();
    fs::write(model_dir.join("weights.bin"), [0_u8; 7]).unwrap();

    let models = scan_lmstudio_models(&root).unwrap();

    fs::remove_dir_all(&root).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider, "lmstudio");
    assert_eq!(
        models[0].name,
        "lmstudio-community/Qwen3-Coder-Next-MLX-4bit"
    );
    assert_eq!(models[0].size_bytes, Some(7));
}

#[test]
fn detects_loaded_llamacpp_model_from_process_args() {
    let processes = vec![ProcessInfo {
        pid: 42,
        cpu_percent: Some(12.5),
        memory_bytes: Some(2048),
        command: "/usr/local/bin/llama-server -m /models/foo.gguf --port 8080".to_string(),
    }];

    let loaded = detect_llamacpp_loaded(&processes);

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].provider, "llama.cpp");
    assert_eq!(loaded[0].name, "/models/foo.gguf");
    assert_eq!(loaded[0].status, ModelStatus::Loaded);
    assert_eq!(loaded[0].pid, Some(42));
}

#[test]
fn remove_prints_provider_specific_command_without_deleting() {
    assert_eq!(
        remove_command("ollama", "qwen3.6:27b").unwrap(),
        "ollama rm qwen3.6:27b"
    );
    assert_eq!(
        remove_command("lmstudio", "lmstudio-community/foo").unwrap(),
        "lms unload lmstudio-community/foo && rm -rf ~/.lmstudio/models/lmstudio-community/foo"
    );
    assert_eq!(
        remove_command("llama.cpp", "/models/foo.gguf").unwrap(),
        "rm /models/foo.gguf"
    );
}
