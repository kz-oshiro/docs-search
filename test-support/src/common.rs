use crate::{random::Random, *};
use serde_json::json;
fn t(key: &str, args: &[&str]) -> String {
    template("common", key, args)
}
pub fn column(mut n: usize) -> String {
    let mut result = String::new();
    while n > 0 {
        n -= 1;
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    result
}
pub fn cell(address: &str, value: impl std::fmt::Display, kind: &str) -> String {
    let value = xml(value);
    match kind {
        "inlineStr" => t("cell-inline", &[address, &value]),
        "formula" => t("cell-formula", &[address, &value]),
        _ => format!(
            "<c r=\"{address}\"{}><v>{value}</v></c>",
            if kind == "n" {
                String::new()
            } else {
                format!(" t=\"{kind}\"")
            }
        ),
    }
}
fn worksheet(n: usize, role: &str, rng: &mut Random, rows: usize, cols: usize) -> String {
    let mut first = if role == "needle" && n == 1 {
        vec![
            cell("A1", 0, "s"),
            cell("B1", "Inline needle", "inlineStr"),
            cell("C1", 42, "n"),
            cell("D1", "needle", "formula"),
            cell("E1", 1, "b"),
        ]
    } else if role == "needle" && n == 2 {
        vec![cell("A1", "hidden-beacon", "inlineStr")]
    } else if role == "macro" && n == 1 {
        vec![cell("A1", "BEACON macro value", "inlineStr")]
    } else {
        vec![cell(
            "A1",
            format!("Document sheet {n} inventory"),
            "inlineStr",
        )]
    };
    for c in first.len() + 1..=cols {
        first.push(cell(
            &format!("{}1", column(c)),
            format!("field {c:02}"),
            "inlineStr",
        ));
    }
    let mut lines = vec![t("first-row", &[&first.join("")])];
    let start = if role == "needle" && n == 1 {
        let values: String = (2..=cols)
            .map(|c| cell(&format!("{}2", column(c)), rng.number(), "n"))
            .collect();
        lines.push(format!(
            "<row r=\"2\"><c r=\"A2\" t=\"s\"><v>1</v></c>{values}</row>"
        ));
        3
    } else {
        2
    };
    for row in start..=rows {
        let values: String = (1..=cols)
            .map(|c| {
                let value = rng.number();
                let address = format!("{}{row}", column(c));
                if [1, 4, 8, 12].contains(&c) {
                    cell(
                        &address,
                        format!("record {n:02} {row:03} {c:02} reference {value}"),
                        "inlineStr",
                    )
                } else {
                    cell(&address, value, "n")
                }
            })
            .collect();
        lines.push(t("data-row", &[&row.to_string(), &values]));
    }
    let drawing = if role == "needle" && n == 1 {
        format!("<drawing xmlns:r=\"{R}\" r:id=\"rIdDrawing\"/><legacyDrawing xmlns:r=\"{R}\" r:id=\"rIdVml\"/>")
    } else {
        String::new()
    };
    t(
        "worksheet",
        &[
            S,
            &column(cols),
            &rows.to_string(),
            &lines.join(""),
            &drawing,
        ],
    )
}
pub fn workbook(
    path: &Path,
    role: &str,
    seed: u64,
    sheets: usize,
    rows: usize,
    cols: usize,
) -> Result<()> {
    let mut rng = Random::new(&format!(
        "{seed}:{role}:{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let main = if path.extension().is_some_and(|s| s == "xlsm") {
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml"
    } else {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    };
    let mut overrides: String = (1..=sheets)
        .map(|i| t("worksheet-type", &[&i.to_string()]))
        .collect();
    if role == "needle" {
        overrides += &t("drawing-types", &[]);
    }
    let sheet_entries: String = (1..=sheets)
        .map(|i| {
            let name = if i <= 3 {
                ["Main", "Hidden", "Data"][i - 1].into()
            } else {
                format!("Data{i:02}")
            };
            t(
                "sheet",
                &[
                    &name,
                    &i.to_string(),
                    &i.to_string(),
                    if role == "needle" && i == 2 {
                        " state=\"hidden\""
                    } else {
                        ""
                    },
                ],
            )
        })
        .collect();
    let mut rels = t("rels-start", &[P]);
    for i in 1..=sheets {
        rels += &t("worksheet-rel", &[&i.to_string(), R, &i.to_string()]);
    }
    if role == "needle" {
        rels += &t("shared-string-rel", &[R]);
    }
    rels += "</Relationships>";
    let mut entries = vec![];
    put(
        &mut entries,
        "[Content_Types].xml",
        t("workbook-types", &[CT, main, &overrides]),
    );
    put(&mut entries, "_rels/.rels", t("workbook-root", &[P, R]));
    put(
        &mut entries,
        "xl/workbook.xml",
        t("workbook-start", &[S, R]) + &sheet_entries + "</sheets></workbook>",
    );
    put(&mut entries, "xl/_rels/workbook.xml.rels", rels);
    if role == "needle" {
        put(
            &mut entries,
            "xl/sharedStrings.xml",
            t("shared-strings", &[S]),
        );
        put(
            &mut entries,
            "xl/worksheets/_rels/sheet1.xml.rels",
            t("sheet-drawing-rels", &[P, R, R]),
        );
        put(
            &mut entries,
            "xl/drawings/drawing1.xml",
            t("drawing", &[XDR, A]),
        );
        put(&mut entries, "xl/drawings/vmlDrawing1.vml", t("vml", &[]));
    }
    for i in 1..=sheets {
        put(
            &mut entries,
            &format!("xl/worksheets/sheet{i}.xml"),
            worksheet(i, role, &mut rng, rows, cols),
        );
    }
    package(path, entries, false, false)
}
fn split_runs(text: &str, split: bool, key: &str) -> String {
    if !split {
        t(key, &[&xml(text)])
    } else {
        let at = text.char_indices().nth(3).map_or(text.len(), |(i, _)| i);
        t(key, &[&xml(&text[..at])]) + &t(key, &[&xml(&text[at..])])
    }
}
fn shape(name: &str, text: &str, id: usize, split: bool) -> String {
    t(
        "shape",
        &[
            &id.to_string(),
            &xml(name),
            &split_runs(text, split, "drawing-run"),
        ],
    )
}
pub fn presentation(path: &Path, seed: u64, slides: usize, shapes: usize) -> Result<()> {
    let mut rng = Random::new(&format!(
        "{seed}:{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let ids: String = (1..=slides)
        .map(|i| t("slide-id", &[&(255 + i).to_string(), &i.to_string()]))
        .collect();
    let group = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>";
    let colors: String = [
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "202020"),
        ("lt2", "EEEEEE"),
        ("accent1", "4472C4"),
        ("accent2", "ED7D31"),
        ("accent3", "A5A5A5"),
        ("accent4", "FFC000"),
        ("accent5", "5B9BD5"),
        ("accent6", "70AD47"),
        ("hlink", "0563C1"),
        ("folHlink", "954F72"),
    ]
    .into_iter()
    .map(|(name, value)| t("theme-color", &[name, value, name]))
    .collect();
    let mut types = t("presentation-types-start", &[CT]);
    let mut rels = t("presentation-rels-start", &[P, R]);
    for i in 1..=slides {
        types += &t("slide-type", &[&i.to_string()]);
        rels += &t("slide-rel", &[&i.to_string(), R, &i.to_string()]);
    }
    types += "</Types>";
    rels += "</Relationships>";
    let mut entries = vec![];
    for (name, text) in [
        ("[Content_Types].xml", types),
        ("_rels/.rels", t("presentation-root", &[P, R])),
        ("ppt/presentation.xml", t("presentation", &[PPT, R, &ids])),
        ("ppt/_rels/presentation.xml.rels", rels),
        (
            "ppt/slideMasters/slideMaster1.xml",
            t("slide-master", &[PPT, A, R, group]),
        ),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            t("master-rels", &[P, R, R]),
        ),
        (
            "ppt/slideLayouts/slideLayout1.xml",
            t("slide-layout", &[PPT, A, group]),
        ),
        (
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            t("layout-rels", &[P, R]),
        ),
        ("ppt/theme/theme1.xml", t("theme", &[A, &colors])),
    ] {
        put(&mut entries, name, text);
    }
    for slide in 1..=slides {
        let mut items = String::new();
        for position in 1..=shapes {
            let special = if position == 1 {
                match slide {
                    1 => Some("Beacon presentation title"),
                    2 => Some("BEACON split phrase"),
                    4 => Some("Beacon hidden slide"),
                    _ => None,
                }
            } else {
                None
            };
            let text = special.map(str::to_owned).unwrap_or_else(|| {
                format!(
                    "Slide {slide:02} topic {position:02} evidence {} for regional planning",
                    rng.number()
                )
            });
            items += &shape(
                &format!("Text {position}"),
                &text,
                position + 1,
                slide == 2 && position == 1,
            );
        }
        if slide == 3 {
            items += &t("table", &[]);
        }
        let text = t("slide-start", &[PPT, A])
            + if slide == 4 { " show=\"0\"" } else { "" }
            + &t("slide-body", &[group, &items]);
        put(&mut entries, &format!("ppt/slides/slide{slide}.xml"), text);
        put(
            &mut entries,
            &format!("ppt/slides/_rels/slide{slide}.xml.rels"),
            t("slide-rels", &[P, R]),
        );
    }
    package(path, entries, false, false)
}
fn paragraph(text: &str, split: bool) -> String {
    "<w:p>".to_owned() + &split_runs(text, split, "word-run") + "</w:p>"
}
pub fn word_document(path: &Path, seed: u64, paragraphs: usize) -> Result<()> {
    let mut rng = Random::new(&format!(
        "{seed}:{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let mut body = paragraph("Beacon field guide", false) + &paragraph("BEACON followup", true);
    for i in 3..=paragraphs {
        body += &paragraph(&format!("Section {i:03}: regional operations record {}. This paragraph carries stable sample content for search and extraction.",rng.number()),false);
    }
    let start =
        "<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"5000\"/></w:tblGrid><w:tr><w:tc><w:tcPr/>";
    body += &(start.to_owned()
        + &paragraph("Beacon table entry", false)
        + start
        + &paragraph("Beacon nested table", false)
        + "</w:tc></w:tr></w:tbl>"
        + &paragraph("End of table cell", false)
        + "</w:tc></w:tr></w:tbl>");
    package(
        path,
        vec![
            (
                "[Content_Types].xml".into(),
                t("word-types", &[CT]).into_bytes(),
            ),
            ("_rels/.rels".into(), t("word-root", &[P, R]).into_bytes()),
            (
                "word/document.xml".into(),
                t("word-document", &[W, &body]).into_bytes(),
            ),
        ],
        false,
        false,
    )
}
pub fn text_document(path: &Path, seed: u64, lines: usize, bom: bool) -> Result<()> {
    let mut rng = Random::new(&format!(
        "{seed}:{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let mut content: Vec<_> = (1..=lines).map(|i|format!("Record {i:03}: project status and notes {} with enough words to exercise long line previews.",rng.number())).collect();
    content[0] = if bom { "BOM beacon" } else { "Beacon alpha" }.into();
    if !bom {
        for (i, text) in [
            (24, "beacon beta"),
            (69, "literal a+b expression"),
            (70, "decomposed cafe\u{301} and Straße"),
            (99, "UTF-8 の日本語サンプル"),
        ] {
            content[i] = text.into();
        }
    }
    write(
        path,
        format!(
            "{}{}\n",
            if bom { "\u{feff}" } else { "" },
            content.join("\n")
        ),
    )
}
pub fn generate(output: &Path, profile: &str) -> Result<Value> {
    if !["acceptance", "load"].contains(&profile) {
        return Err("profile must be acceptance or load".into());
    }
    if output.exists() && fs::read_dir(output)?.next().is_some() {
        return Err("output directory must be empty".into());
    }
    let root = output.join("search");
    workbook(
        &root.join("spreadsheets/needle-book.xlsx"),
        "needle",
        SEED,
        3,
        295,
        12,
    )?;
    workbook(
        &root.join("spreadsheets/filename-token.xlsx"),
        "plain",
        SEED,
        3,
        295,
        12,
    )?;
    workbook(
        &root.join("nested/quarterly-ledger.xlsm"),
        "macro",
        SEED,
        3,
        295,
        12,
    )?;
    presentation(&root.join("nested/team-briefing.pptx"), SEED, 165, 8)?;
    word_document(&root.join("documents/operations-guide.docx"), SEED, 3400)?;
    text_document(&root.join("notes/research-log.txt"), SEED, 150, false)?;
    text_document(&root.join("notes/bom-note.txt"), SEED, 150, true)?;
    for (name, text) in [
        ("notes/shift-jis.txt", "Shift_JIS の文書\n日本語①を検索\n"),
        (
            "code/shift-jis.html",
            "<meta charset=\"Shift_JIS\">\n<p>日本語①を検索</p>\n",
        ),
    ] {
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
        assert!(!errors);
        write(&root.join(name), bytes)?;
    }
    // These are documents to search, including the Python sample; no interpreter is required.
    let documents: Value = serde_json::from_str(include_str!("../templates/documents.json"))?;
    for (name, content) in documents.as_object().unwrap() {
        write_text(&root.join(name), content.as_str().unwrap())?;
    }
    for (name, bytes) in [
        (
            "spreadsheets/broken.xlsx",
            b"not an OOXML archive".as_slice(),
        ),
        ("notes/invalid-utf8.txt", b"invalid utf8: \xff\xfe\n"),
        (
            "errors/broken-presentation.pptx",
            b"not a PowerPoint archive",
        ),
        ("errors/broken-document.docx", b"not a Word archive"),
        ("errors/invalid-bom.txt", b"\xef\xbb\xbf\xff\n"),
        (
            "spreadsheets/~$temporary.xlsx",
            b"excluded office temporary file",
        ),
        ("notes/legacy.xls", b"unsupported format beacon"),
    ] {
        write(&root.join(name), bytes)?;
    }
    if profile == "load" {
        for i in 1..=32 {
            workbook(
                &output.join(format!("load/book-{i:03}.xlsx")),
                "load",
                SEED,
                5,
                150,
                16,
            )?;
            presentation(
                &output.join(format!("load/slides-{i:03}.pptx")),
                SEED,
                165,
                8,
            )?;
        }
        for i in 1..=31 {
            word_document(&output.join(format!("load/report-{i:03}.docx")), SEED, 3400)?;
        }
        for i in 1..=20 {
            text_document(
                &output.join(format!("load/notes-{i:03}.txt")),
                SEED,
                1000,
                false,
            )?;
        }
    }
    write(
        &output.join("backend-cases.json"),
        include_bytes!("../../tests/fixtures/backend-cases.json"),
    )?;
    let inventory: Vec<_> = files(output)?
        .into_iter()
        .filter(|s| s != "backend-cases.json")
        .collect();
    let manifest = json!({"schemaVersion":1,"profile":profile,"seed":SEED,"files":inventory});
    write_json(&output.join("manifest.json"), &manifest)?;
    Ok(manifest)
}
