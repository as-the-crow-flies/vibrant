const RADIX: u32 = 256;

fn extract_byte(value: u32, shift: u32) -> u32 {
    return (value >> shift) & 255;
}
