# VIBRANT — Visualizing Brain Anatomy & Tractography

<p align="center">
  <a href="https://as-the-crow-flies.github.io/vibrant/">
    <img src="cover.png" alt="Screenshot of the VIBRANT application" width="75%" height=75% />
  </a>
</p>

<p align="center">
  <strong>Web-based Cinematic Visualization for Neuroanatomy and Tractography</strong>
</p>

<p align="center">
  <a href="https://as-the-crow-flies.github.io/vibrant/">🌐 Launch Web App 🌐</a>
</p>

## Overview

[VIBRANT](https://as-the-crow-flies.github.io/vibrant/) is an interactive visualization tool for neuroanatomy and tractography, designed for high-quality real-time rendering directly in the browser or as a native desktop application.

It supports:

- 🧵 Cinematic tractography rendering
- 🎨 Tract scalar overlays (`.tsf`)
- 🧠 NIfTI volume rendering
- ✂️ Volume masking workflows
- 🌍 HDR environment lighting
- ⚡ Real-time rendering using Rust and WebGPU

# Features

## 🧵 Tractography Rendering

Load any number of `.tck` tractography files and render them with vibrant coloring and shadows.

Tracts can be colored using:

- **Tangent direction** *(default)*
- **Per-bundle colors**
- **Tract scalar files (`.tsf`)**

### 🎨 Tract Scalar File Support

After loading `.tck` files, VIBRANT automatically applies matching `.tsf` scalar files. Several sequential colormaps are supported.

Example:

```text
AF_Left.tck
AF_Left_fa.tsf
```

| Tangent Coloring | Bundle Coloring | Scalar Coloring |
| --- | --- | --- |
| ![](img/tractography-tangent.png) | ![](img/tractography-color.png) | ![](img/tractography-scalar.png) |

## 🧠 NIfTI Volume Rendering

Load scalar `.nii` and `.nii.gz` files for high-quality volume rendering with adjustable material properties.

Multiple NIfTI volumes can be loaded simultaneously, making it easy to visualize:

- Anatomical scans
- ROIs
- Segmentations
- Functional overlays

### ✂️ Masking Support

Any NIfTI file containing `mask` in its filename (e.g. `brain_mask.nii.gz`) is automatically imported as a mask.

Masks can then be assigned to one or more volumes.

Supported mask types:

- **Binary Masks** — classic voxel masking
- **Signed Distance Field Masks** — useful for erosion effects

| Volume Rendering | Cinematic Rendering |
| --- | --- |
| ![](img/volume.png) | ![](img/volume-cinematic.png) |

## 🌍 (Experimental) Environment Maps

Load `.exr` HDR environment textures to illuminate scenes using image-based lighting.

This enables more realistic cinematic rendering and soft reflections.

HDRIs from resources like [Poly Haven](https://polyhaven.com/hdris/studio) work well.

# Getting Started

## 🌐 Web Application

VIBRANT runs directly in the browser using **WebGPU** for hardware-accelerated rendering.

👉 https://as-the-crow-flies.github.io/vibrant/

## 💻 Running Natively

VIBRANT can also run natively on:

- Windows
- macOS
- Linux

VIBRANT is written in **Rust**, making it portable and easy to build:

### 1. Install Rust

https://rustup.rs/

### 2. Clone the Repository

```bash
git clone git@github.com:as-the-crow-flies/vibrant.git
```

### 3. Enter the Project Directory

```bash
cd vibrant
```

### 4. Build & Run

```bash
cargo run
```

# Citation

If you use VIBRANT in academic work, please cite [Our Paper](https://doi.org/10.1111/cgf.70372):

```bibtex
@article{https://doi.org/10.1111/cgf.70372,
  author = {Kraaijeveld, B. and Jalba, A.C. and Vilanova, A. and Chamberland, M.},
  title = {Real-Time Rendering of Dynamic Line Sets using Voxel Ray Tracing},
  journal = {Computer Graphics Forum},
  volume = {n/a},
  number = {n/a},
  pages = {e70372},
  keywords = {CCS Concepts, • Computing methodologies → Visibility, Ray tracing, Rasterization, • Human-centered computing → Scientific visualization},
  doi = {https://doi.org/10.1111/cgf.70372},
  url = {https://onlinelibrary.wiley.com/doi/abs/10.1111/cgf.70372},
  eprint = {https://onlinelibrary.wiley.com/doi/pdf/10.1111/cgf.70372}
}
```

# Acknowledgements

This work was funded by the Dutch Research Council (NWO), grant **OCENW.M.22.352**, awarded to Maxime Chamberland.
