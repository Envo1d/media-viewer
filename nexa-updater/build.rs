fn main() {
    let key_hex = std::env::var("NEXA_PUBLIC_KEY").unwrap_or_else(|_| {
        eprintln!(
            "cargo:warning=NEXA_PUBLIC_KEY is not set. \
             Using the all-zeros DEV placeholder — \
             signatures will NOT verify against real releases."
        );
        "0".repeat(64)
    });

    assert!(
        key_hex.len() == 64 && key_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "NEXA_PUBLIC_KEY must be exactly 64 lowercase hex characters (32 bytes)."
    );

    println!("cargo:rustc-env=NEXA_PUBLIC_KEY={key_hex}");
    println!("cargo:rerun-if-env-changed=NEXA_PUBLIC_KEY");

    #[cfg(target_os = "windows")]
    embed_resources();
}

#[cfg(target_os = "windows")]
fn embed_resources() {
    use std::path::Path;

    let ico_src = Path::new("../nexa/assets/icons/icon.ico");
    let png_src = Path::new("../nexa/assets/icons/icon.png");

    println!("cargo:rerun-if-changed=../nexa/assets/icons/icon.ico");
    println!("cargo:rerun-if-changed=../nexa/assets/icons/icon.png");

    let icon_path = if ico_src.exists() {
        ico_src.to_path_buf()
    } else if png_src.exists() {
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        let generated = Path::new(&out_dir).join("icon_generated.ico");
        let png = std::fs::read(png_src).expect("failed to read icon.png");
        std::fs::write(&generated, png_to_ico_bytes(&png)).expect("failed to write generated icon");
        generated
    } else {
        eprintln!("cargo:warning=No icon found — PE will have no embedded icon.");
        return;
    };

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_str().expect("icon path not UTF-8"));
    res.set("FileDescription", "Nexa Updater");
    res.set("ProductName", "Nexa");
    res.set("LegalCopyright", "© 2026 Envo1d");
    res.set("CompanyName", "Envo1d");
    res.set("InternalName", "nexa-updater");
    res.set("OriginalFilename", "nexa-updater.exe");

    res.set_manifest(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 / 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>"#,
    );

    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=winres failed: {e}");
    }
}

fn png_to_ico_bytes(png: &[u8]) -> Vec<u8> {
    assert!(
        png.len() >= 24 && png[1..4] == *b"PNG",
        "icon.png is not valid PNG"
    );
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    let ico_w = if width >= 256 { 0u8 } else { width as u8 };
    let ico_h = if height >= 256 { 0u8 } else { height as u8 };
    let data_offset: u32 = 6 + 16; // header + 1 entry
    let data_size: u32 = png.len() as u32;
    let mut ico = Vec::with_capacity(data_offset as usize + png.len());
    ico.extend_from_slice(&[0, 0]);
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
