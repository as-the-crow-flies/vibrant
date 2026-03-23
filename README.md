# VIBRANT

-------

## Prerequisites

1. Install Rust: <https://rustup.rs/>
2. For video export, install `ffmpeg` and make sure it is available on your `PATH`

## How to Run

Run the application from the `vibrant/` project root:

```bash
cargo run -- [OPTIONS]
```

For faster exports, you can use release mode:

```bash
cargo run --release -- [OPTIONS]
```

If you pass no export flags, VIBRANT starts in interactive mode.

## Common Usage

### Launch the interactive viewer

```bash
cargo run -- -i data/tracts.tck
```

### Load multiple input files

```bash
cargo run -- -i data/tracts.tck -i data/surface.obj -i data/volume.nii.gz
```

### Save a screenshot

```bash
cargo run -- -i data/tracts.tck --screenshot output/screenshot.png
```

### Render a video

```bash
cargo run -- -i data/tracts.tck --video output/turntable.mp4
```

## CLI Reference

VIBRANT uses flags rather than subcommands. The table below documents the full CLI surface currently implemented in `src/cli.rs`.

| Option | Argument | Description | Default | Notes |
| --- | --- | --- | --- | --- |
| `-i`, `--input` | `<PATH>` | Input file to load. Supported file types are `.tck`, `.obj`, and `.nii.gz`. | none | Repeat the flag to load multiple files. |
| `--screenshot` | `<PATH>` | Save a screenshot and exit. | none | Cannot be used together with `--video`. |
| `--video` | `<PATH>` | Render a video and exit. | none | Cannot be used together with `--screenshot`. |
| `--fps` | `<INT>` | Video frame rate. | `30` | Only meaningful with `--video`. Must be at least `1`. |
| `--duration` | `<INT>` | Video duration in seconds. | `10` | Only meaningful with `--video`. Must be at least `1`. |
| `--auto-rotate` | none | Enable automatic camera rotation. | `false` | Works in interactive, screenshot, and video workflows. |
| `--rotate-speed` | `<FLOAT>` | Auto-rotation speed in degrees per second. | `10.0` | Only has an effect when `--auto-rotate` is enabled. |
| `--disable-visual-effects` | none | Disable bloom. | `false` | Applies wherever the flag is used, including export workflows. |
| `--zoom` | `<FLOAT>` | Override camera distance for exported output. | none | Applied in screenshot and video modes. |
| `-h`, `--help` | none | Print CLI help. | none | Built in by `clap`. |
| `-V`, `--version` | none | Print the application version. | none | Built in by `clap`. |

## Screenshot Workflow

Use `--screenshot` to render a single frame to disk and exit:

```bash
cargo run -- -i data/tracts.tck --screenshot output/tracts.png
```

You can combine it with export-related camera options:

```bash
cargo run -- \
  -i data/tracts.tck \
  --screenshot output/tracts-closeup.png \
  --zoom 1.5 \
  --disable-visual-effects
```

## Video Workflow

Use `--video` to render frames and encode them through `ffmpeg`:

```bash
cargo run -- -i data/tracts.tck --video output/tracts.mp4
```

A more controlled example with frame rate, duration, and camera settings:

```bash
cargo run -- \
  -i data/tracts.tck \
  --video output/tracts-turntable.mp4 \
  --fps 60 \
  --duration 8 \
  --auto-rotate \
  --rotate-speed 18 \
  --zoom 1.2 \
  --disable-visual-effects
```

Notes:

- Video mode runs with a hidden window and exits after writing all frames once assets are loaded.
- The current encoder uses `ffmpeg` with `libx264`, so `.mp4` is the most practical output format to use in examples.
- If inputs are missing or fail to load, export may never complete because the renderer waits for assets.
