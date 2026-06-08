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

From a local clone:

```sh
git clone git@github.com:alexazevedo/llmctl.git
cd llmctl
cargo test
cargo build --release
./target/release/llmctl
```

Rust is required. Install it with:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Install From Source

Install directly from the local repository:

```sh
cargo install --path .
llmctl
```

This installs `llmctl` into Cargo's bin directory, usually `~/.cargo/bin`.
Make sure that directory is in your `PATH`.

You can also install the release binary manually:

```sh
cargo build --release
mkdir -p ~/.local/bin
cp target/release/llmctl ~/.local/bin/llmctl
llmctl
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

## Publishing A Release

CI runs on every push or pull request to `main`.

Releases are built from version tags. To publish a release:

```sh
git checkout main
git pull
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

The `release` GitHub Actions workflow builds release archives and attaches them
to the GitHub release for that tag.

You can also run the release workflow manually from GitHub:

1. Open the repository on GitHub.
2. Go to `Actions`.
3. Select the `release` workflow.
4. Click `Run workflow`.
5. Enter an existing tag such as `v0.1.0`.

The manual workflow expects the tag to already exist. Prefer tag-based releases
for normal publishing because tags make the release source immutable and easy to
audit.

## Local llama.cpp Model Roots

`llama.cpp` does not have a universal model registry. To include standalone GGUF
files in `list` and `storage`, configure scan roots:

```sh
LLMCTL_GGUF_DIRS="$HOME/models:$HOME/llama.cpp/models" llmctl list
```
