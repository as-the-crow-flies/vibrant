use std::{path::PathBuf, process::Command};

use tempfile::tempdir;

fn screenshot_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("screenshots/cli_zoom_test.obj")
}

#[test]
fn cli_screenshot_clears_post_buffer_background() {
    let fixture = screenshot_fixture_path();
    assert!(
        fixture.exists(),
        "missing screenshot fixture: {}",
        fixture.display()
    );

    let temp = tempdir().expect("create temp dir");
    let screenshot = temp.path().join("cli_zoom_test.png");

    let status = Command::new(env!("CARGO_BIN_EXE_vibrant-app"))
        .args([
            "--input",
            fixture.to_str().expect("fixture path utf-8"),
            "--screenshot",
            screenshot.to_str().expect("output path utf-8"),
        ])
        .status()
        .expect("run screenshot regression binary");

    assert!(status.success(), "screenshot regression command failed");
    assert!(screenshot.exists(), "screenshot output was not created");

    let assertion = Command::new("python3")
        .arg("-")
        .arg(&screenshot)
        .arg("16")
        .arg(
            r#"from PIL import Image
import sys

path = sys.argv[1]
tolerance = int(sys.argv[2])
img = Image.open(path).convert('RGBA')
w, h = img.size
assert w > 0 and h > 0, 'expected non-empty screenshot dimensions'

row = [img.getpixel((x, 0)) for x in range(w)]
transparent_top = sum(1 for pixel in row if pixel[3] == 0)
black_opaque_top = sum(1 for pixel in row if pixel == (0, 0, 0, 255))
transparent_pixels = 0
visible_pixels = 0

for y in range(h):
    for x in range(w):
        pixel = img.getpixel((x, y))
        if pixel[3] == 0:
            transparent_pixels += 1
        else:
            visible_pixels += 1

assert transparent_top >= w - tolerance, 'top row background should stay transparent aside from a few visible geometry pixels'
assert black_opaque_top == 0, 'top row should not retain an opaque black bar from stale post-buffer contents'
assert transparent_pixels > 0, 'expected transparent background pixels in saved screenshot'
assert visible_pixels > 0, 'expected rendered geometry pixels in saved screenshot'
"#,
        )
        .status()
        .expect("run PNG assertions");

    assert!(assertion.success(), "PNG regression assertions failed");
}
