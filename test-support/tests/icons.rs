use docs_search_test_support::{icons, TempDir};
use std::{fs, io::Read};
#[test]
fn last_visible_shape_wins_through_transparent_shapes() {
    let source = r##"<svg viewBox="0 0 128 128">
        <rect x="0" y="0" width="128" height="128" fill="#110000"/>
        <rect x="32" y="32" width="64" height="64" fill="#0000FF"/>
        <rect x="32" y="32" width="64" height="64" fill="#00FF00"/>
        <rect x="0" y="0" width="128" height="128" fill="none"/>
    </svg>"##;
    let pixels = icons::rasterize(source, 4).unwrap();
    for (index, pixel) in pixels.chunks_exact(4).enumerate() {
        let expected = if [5, 6, 9, 10].contains(&index) {
            [0, 255, 0, 255]
        } else {
            [17, 0, 0, 255]
        };
        assert_eq!(pixel, expected, "pixel {index}");
    }
}
#[test]
fn approved_pixels_and_icon_directory() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("src-tauri/icons");
    let temp = TempDir::new("icons");
    fs::copy(root.join("icon.svg"), temp.path().join("icon.svg")).unwrap();
    icons::generate(temp.path()).unwrap();
    let first = fs::read(temp.path().join("icon.ico")).unwrap();
    let png = fs::read(temp.path().join("icon.png")).unwrap();
    icons::generate(temp.path()).unwrap();
    assert_eq!(first, fs::read(temp.path().join("icon.ico")).unwrap());
    assert_eq!(png, fs::read(temp.path().join("icon.png")).unwrap());
    let approved = fs::read(root.join("icon.ico")).unwrap();
    assert_eq!(
        first, approved,
        "ICO pixels and directory must preserve approved mark"
    );
    // Independently decode PNG's stored scanlines so zlib implementation bytes can differ.
    fn rows(bytes: &[u8]) -> Vec<u8> {
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let mut offset = 8;
        let mut compressed = vec![];
        while offset < bytes.len() {
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            let kind = &bytes[offset + 4..offset + 8];
            let data = &bytes[offset + 8..offset + 8 + length];
            if kind == b"IDAT" {
                compressed.extend(data);
            }
            offset += length + 12;
        }
        let mut out = vec![];
        flate2::read::ZlibDecoder::new(compressed.as_slice())
            .read_to_end(&mut out)
            .unwrap();
        out
    }
    assert_eq!(
        rows(&png),
        rows(&fs::read(root.join("icon.png")).unwrap()),
        "PNG pixel rows must preserve approved mark"
    );
    assert_eq!(u16::from_le_bytes(first[4..6].try_into().unwrap()), 8);
    for (i, size) in [16u8, 20, 24, 32, 48, 64, 128, 0].iter().enumerate() {
        assert_eq!(first[6 + i * 16], *size);
        assert_eq!(first[7 + i * 16], *size);
    }
    assert!(icons::rasterize("<svg viewBox=\"0 0 128 128\"><path/></svg>", 16).is_err());
    assert!(icons::rasterize("<svg viewBox=\"0 0 10 10\"/>", 16).is_err());
}
