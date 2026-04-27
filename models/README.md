# Models directory

This folder holds the Whisper GGML model files (`.bin` / `.gguf`).
These files are **not tracked by git** (see `.gitignore`).

## Download a model

```bash
# Linux / macOS
bash scripts/download_model.sh

# Windows PowerShell
.\scripts\download_model.ps1
```

## Available tiers

| Model  | Size    | Latency (CPU) | Notes                    |
|--------|---------|---------------|--------------------------|
| tiny   | ~75 MB  | ~80 ms        | Fast, English-focused     |
| base ★ | ~142 MB | ~200 ms       | Recommended default       |
| small  | ~466 MB | ~400 ms       | High accuracy             |
| medium | ~1.5 GB | ~900 ms       | Near-perfect, power users |

The `★ base` model is the default. Place the downloaded `.bin` file here
and set `model_path` in `~/.config/voca/config.toml`.
