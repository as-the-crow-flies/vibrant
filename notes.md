Streamline Rasterization ??
1. Rasterize Streamlines to Voxel Grid
2. Find Extinction Depth in Screen Space -> MipMap
  - How to guarrantee a voxel is really 'filled' and thus can be used for culling?
3. Rasterize Streamlines to Screen -> Culling those behind extiction Depth

Visibility Buffer: atomic<u32>
1. Create Depth Slices, front to back, fewer depth bits per pass
2. Render all Vertices, but cull (using parallel prefix sum) if
  1. Not in current Depth Slice
  2. Behind Max Depth Hierarcy
3. After Rendering each Interval, Compute Max Depth (+ Opacity) Hierarchy

4-Pass Rendering
- Depth: u8
- Normal: u16 (only store x,y -> compute z)
- Opacity: u8

Separate, single pixel pass on click (Depth Test 'Equal')
- LineId: u32

Depth Hierarchy
- Depth
- Opacity
