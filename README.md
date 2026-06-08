# llmctl

Local LLM inventory for macOS.

`llmctl` lists models installed in Ollama and LM Studio, shows loaded/running
models when the provider exposes them, reports process CPU/memory usage, and
prints provider-specific removal commands without deleting anything.

## Commands

```sh
llmctl
llmctl list
llmctl running
llmctl storage
llmctl remove <provider> <model>
llmctl doctor
```

## Providers

- Ollama
  - Installed models: `ollama list`, with fallback to `~/.ollama/models/manifests`
  - Loaded models: `ollama ps`
  - Remove command: `ollama rm <model>`
- LM Studio
  - Installed models: `~/.lmstudio/models/<publisher>/<model>`
  - Loaded models: best-effort process detection
  - Remove command: prints `lms unload <model>` plus the model directory removal command
- llama.cpp
  - Loaded models: detects `llama-server`/`llama-cli` processes and `.gguf` arguments
  - Installed models: set `LLMCTL_GGUF_DIRS` to one or more directories
  - Remove command: `rm <path-to-gguf>`

## Build

```sh
cargo build --release
./target/release/llmctl
```

## Install From GitHub Release

After publishing release artifacts, install with:

```sh
curl -fsSL https://raw.githubusercontent.com/aazevedo/llmctl/main/install.sh | sh
```

Override the repository or version:

```sh
curl -fsSL https://raw.githubusercontent.com/aazevedo/llmctl/main/install.sh | \
  LLMCTL_REPO=your-user/llmctl LLMCTL_VERSION=v0.1.0 sh
```

## Local llama.cpp Model Roots

`llama.cpp` does not have a universal model registry. To include standalone GGUF
files in `list` and `storage`, configure scan roots:

```sh
LLMCTL_GGUF_DIRS="$HOME/models:$HOME/llama.cpp/models" llmctl list
```
