# Vibrant

Visualizing Brain Anatomy and Tractography... Vibrantly!

![Screenshot of Vibrant Application](cover.png)

## Getting Started

1. Install Rust using https://rustup.rs/
2. Clone Git Repository `git clone git@github.com:as-the-crow-flies/vibrant.git`
3. Build & Run the application using `cargo run`

## Tractography Rendering

Load any number of `.tck` tract files to render them with vibrant colors and shadows.
Tracts can be colored using their tangent (default), a color per file, or using a `.tsf` tract scalar file.

### (Experimental) Tract Scalar File Support

After loading `.tck` files, load any `.tsf` files with corresponding names to apply the scalar values to this tract, e.g. `test_AF_Left_fa.tsf` will be automatically applied to `AF_Left.tck`. A few sequential colormaps are supported.

## NIfTI Volume Rendering

Load any scalar `.nii.gz` volume file to render it with given contrast and material. Load multiple NIfTI files to show ROIs or segmentations.

### Masking

Load any `*mask*.nii.gz` NIfTI file with 'mask' anywhere in the name to import a mask. Then assign it to one or more volumes. Two types of masks are supported:

- Binary Masks - the classic mask we all know and love
- Signed Distance Field Masks - allowing for e.g. eroding the brain surface

### (Experimental) Environment Maps

By loading a `.exr` HDRI environment texture (e.g. from [Poly Haven](https://polyhaven.com/hdris/studio)), the application uses it as a light source to render your with colored lighting.

## Known Limitations

- All loaded NIfTI files should have the exact same transform. The application cannot handle different volume resolutions at once yet.
