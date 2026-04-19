fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_icon();
    }
}

fn embed_icon() {
    use std::path::Path;

    let ico_src = Path::new("assets/icons/icon.ico");
    let png_src = Path::new("assets/icons/icon.png");

    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    println!("cargo:rerun-if-changed=assets/icons/icon.png");

    let icon_path: std::path::PathBuf = if ico_src.exists() {
        ico_src.to_path_buf()
    } else if png_src.exists() {
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
        let generated = Path::new(&out_dir).join("icon_generated.ico");

        let png = std::fs::read(png_src).expect("Failed to read assets/icons/icon.png");
        let ico = png_to_ico_bytes(&png);
        std::fs::write(&generated, &ico).expect("Failed to write generated icon.ico");

        generated
    } else {
        eprintln!(
            "cargo:warning=No icon file found at assets/icons/icon.ico or \
             assets/icons/icon.png — skipping PE icon embedding."
        );
        return;
    };

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_str().expect("icon path is not valid UTF-8"));

    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=winres failed to compile icon resource: {e}");
    }
}
fn png_to_ico_bytes(png: &[u8]) -> Vec<u8> {
    assert!(
        png.len() >= 24 && png[1..4] == *b"PNG",
        "assets/icons/icon.png is not a valid PNG file"
    );

    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());

    let ico_w = if width >= 256 { 0u8 } else { width as u8 };
    let ico_h = if height >= 256 { 0u8 } else { height as u8 };

    let header_size: u32 = 6;
    let entry_size: u32 = 16;
    let data_offset: u32 = header_size + entry_size; // = 22
    let data_size: u32 = png.len() as u32;

    let mut ico = Vec::with_capacity(data_offset as usize + png.len());

    ico.extend_from_slice(&[0u8, 0]);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    ico.push(ico_w);
    ico.push(ico_h);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&data_size.to_le_bytes());
    ico.extend_from_slice(&data_offset.to_le_bytes());

    ico.extend_from_slice(png);

    ico
}
