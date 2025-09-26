VIBRANT
-------

Experimental Line Rendering Software

# How to Run

1. Ensure you have Rust installed on your system: https://rustup.rs/
2. Build and Run using `cargo run`

# User Interface

1. Settings - Opens Settings Side Panel
2. Open - Allows Importing `.tck` and `.obj` line files
3. Screenshot - Captures a Screenshot

## Settings

### Render Mode

- RayTracing - Our Voxel Ray Tracing Method
- RayTracingQuantized - Kanzler et al.
- RasterizationOrderCorrecting - Groß and Gumhold
- Rasterization - Baseline Rasterization

### Display Mode

- Geometry - Render Tube Geometry
- Volume - Render Occupancy Volume

### Voxelization Mode

- Tube - Our Conservative Voxelization Method
- Box - Axis Aligned Bounding Box Voxelization
- Line - DDA Voxelization

### Resolutions

- Volume - Resolution of Occupancy Volume and A-Buffer

### Appearance

- Streamline Radius - Line Radius radius relative to voxel size
- Lighting - How much lighting to apply
- Ambient/Shadow - Contribution of Ambient Occlusion versus Direct Shadows
- Tangent Color - Contribution of Tangent Coloring
- Alpha - Line Alpha for all lines
- Smoothing - Clamping Value for Phone-Wire Anti-Aliasing
- Culling Slices - Number of slices for culling in RasterizationOrderCorrecting
- Workgroups - How many work groups to launch (set equal to number of workgroups in your GPU for optimal performance)
- Enable Culling - Turn Culling on/off
