use crate::{common::cell, *};
use serde_json::json;
fn t(group: &str, key: &str, args: &[&str]) -> String {
    template(group, key, args)
}
fn c(address: &str, text: &str) -> String {
    cell(address, text, "inlineStr")
}
pub fn context(path: &Path) -> Result<()> {
    let middle: String = [
        ("A12", "左側".into()),
        ("B12", "context-needle".into()),
        ("C12", "結合メモ".into()),
        ("E12", "cached-result".into()),
        ("F12", "L".repeat(310)),
        ("Z12", "far-detail".into()),
    ]
    .into_iter()
    .map(|(a, v)| cell(a, v, if a == "E12" { "formula" } else { "inlineStr" }))
    .collect();
    let rows = t("context", "top-row", &[&c("A1", "edge-top")])
        + "<row r=\"11\" hidden=\"1\"/>"
        + &format!("<row r=\"12\">{middle}</row>")
        + &t("context", "bottom-row", &[&c("XFD1048576", "edge-bottom")]);
    let drawing = t(
        "context",
        "drawing",
        &[
            XDR,
            A,
            &t("context", "shape", &["1", "Anchored", "context-shape"]),
            &t(
                "context",
                "shape",
                &["2", "Unanchored", "context-unanchored"],
            ),
        ],
    );
    let mut entries = vec![];
    for (name, text) in [
        ("[Content_Types].xml", t("context", "types", &[CT])),
        ("_rels/.rels", t("context", "root-rel", &[P, R])),
        ("xl/workbook.xml", t("context", "workbook", &[S, R])),
        (
            "xl/_rels/workbook.xml.rels",
            t("context", "workbook-rels", &[P, R]),
        ),
        (
            "xl/worksheets/sheet1.xml",
            t("context", "worksheet", &[S, R, &rows]),
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            t("context", "sheet-rels", &[P, R]),
        ),
        ("xl/drawings/drawing1.xml", drawing),
    ] {
        put(&mut entries, name, text);
    }
    package(path, entries, false, false)
}
type Rows<'a> = Vec<(usize, Vec<(&'a str, &'a str)>)>;
fn condition_workbook(path: &Path, sheets: Vec<(&str, Rows<'_>)>) -> Result<()> {
    let mut types = t("conditions", "types-start", &[CT]);
    let mut book = t("conditions", "workbook-start", &[S, R]);
    let mut rels = t("conditions", "rels-start", &[P]);
    for (index, (name, _)) in sheets.iter().enumerate() {
        let i = (index + 1).to_string();
        types += &t("conditions", "sheet-type", &[&i]);
        book += &t("conditions", "sheet", &[name, &i, &i]);
        rels += &t("conditions", "sheet-rel", &[&i, R, &i]);
    }
    types += "</Types>";
    book += "</sheets></workbook>";
    rels += "</Relationships>";
    let mut entries = vec![];
    for (name, text) in [
        ("[Content_Types].xml", types),
        ("_rels/.rels", t("conditions", "root-rel", &[P, R])),
        ("xl/workbook.xml", book),
        ("xl/_rels/workbook.xml.rels", rels),
    ] {
        put(&mut entries, name, text);
    }
    for (index, (_, rows)) in sheets.into_iter().enumerate() {
        let rows: String = rows
            .into_iter()
            .map(|(n, values)| {
                t(
                    "conditions",
                    "row",
                    &[
                        &n.to_string(),
                        &values.into_iter().map(|(a, v)| c(a, v)).collect::<String>(),
                    ],
                )
            })
            .collect();
        put(
            &mut entries,
            &format!("xl/worksheets/sheet{}.xml", index + 1),
            t("conditions", "worksheet", &[S, &rows]),
        );
    }
    package(path, entries, false, false)
}
pub fn conditions(output: &Path) -> Result<()> {
    condition_workbook(
        &output.join("search/conditions.xlsx"),
        vec![
            (
                "Items",
                vec![
                    (
                        12,
                        vec![("A12", "顧客番号"), ("D12", "必須"), ("E12", "稼働")],
                    ),
                    (13, vec![("A13", "顧客補足")]),
                    (14, vec![("D14", "必須")]),
                    (
                        15,
                        vec![("A15", "顧客番号"), ("D15", "必須"), ("E15", "廃止")],
                    ),
                    (16, vec![("B16", "AND OR")]),
                    (17, vec![("A17", "customer")]),
                ],
            ),
            ("Other", vec![(12, vec![("A12", "顧客番号")])]),
        ],
    )?;
    condition_workbook(
        &output.join("search/filename-signal.xlsx"),
        vec![("Data", vec![(1, vec![("A1", "body-signal")])])],
    )?;
    write_text(&output.join("search/notes.txt"), "顧客\n必須\nAND OR\n")?;
    write(
        &output.join("errors/filename-signal-broken.xlsx"),
        b"not an OOXML archive",
    )
}
pub fn issue_workbook(path: &Path) -> Result<()> {
    let mut entries = vec![];
    for (name, key, args) in [
        ("[Content_Types].xml", "types", vec![]),
        ("_rels/.rels", "root-rel", vec![P, R]),
        ("xl/workbook.xml", "workbook", vec![S, R]),
        (
            "xl/_rels/workbook.xml.rels",
            "workbook-rels",
            vec![P, R, R, R, R],
        ),
        ("xl/sharedStrings.xml", "shared-strings", vec![S]),
        ("xl/worksheets/sheet10.xml", "zeta-sheet", vec![S]),
        ("xl/worksheets/sheet2.xml", "alpha-sheet", vec![S]),
        ("xl/styles.xml", "styles", vec![S]),
    ] {
        put(&mut entries, name, t("issues", key, &args));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    package(path, entries, true, true)
}
pub fn issues(output: &Path) -> Result<()> {
    issue_workbook(&output.join("excel/layout.xlsx"))?;
    for parent in ["left/same", "right/same"] {
        write(
            &output.join(parent).join("multiple.txt"),
            "対象 keep\r\n対象 skip\r\n対象 keep skip\r\n対象 KEEP\r\n",
        )?;
        write(
            &output.join(parent).join("long.txt"),
            format!("\u{feff}{}対象 対象 😀\r\n後文\r\n", "あ".repeat(400)),
        )?;
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode("対象 対象\r\nそのまま\r\n");
        assert!(!errors);
        write(&output.join(parent).join("sjis.txt"), bytes)?;
    }
    for n in 0..205 {
        write(
            &output.join(format!("pages/{n:03}.txt")),
            format!("対象 keep {n}\n"),
        )?;
    }
    Ok(())
}
fn rels(items: &[(&str, &str, &str, bool)]) -> String {
    let mut result = t("office", "rels-start", &[P]);
    for (id, kind, target, external) in items {
        result += &t("office", "rel", &[id, R, kind, &xml(target)]);
        if *external {
            result += " TargetMode=\"External\"";
        }
        result += "/>";
    }
    result + "</Relationships>"
}
fn base(main: &str, kind: &str) -> Entries {
    vec![
        (
            "_rels/.rels".into(),
            rels(&[("root", "officeDocument", main, false)]).into_bytes(),
        ),
        (
            "[Content_Types].xml".into(),
            t("office", "types", &[CT, main, kind]).into_bytes(),
        ),
    ]
}
fn office_workbook(rows: &str, comments: bool, macro_enabled: bool) -> Entries {
    let kind = if macro_enabled {
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml"
    } else {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    };
    let mut entries = base("xl/workbook.xml", kind);
    put(
        &mut entries,
        "xl/workbook.xml",
        t("office", "workbook", &[S, R]),
    );
    put(
        &mut entries,
        "xl/_rels/workbook.xml.rels",
        rels(&[("sheet", "worksheet", "worksheets/data.xml", false)]),
    );
    put(
        &mut entries,
        "xl/worksheets/data.xml",
        t("office", "worksheet-start", &[S, R, rows])
            + if comments {
                "<legacyDrawing r:id=\"vml\"/>"
            } else {
                ""
            }
            + "</worksheet>",
    );
    if comments {
        put(
            &mut entries,
            "xl/worksheets/_rels/data.xml.rels",
            rels(&[
                ("vml", "vmlDrawing", "../drawings/shapes.vml", false),
                ("comment", "comments", "../extra/legacy.xml", false),
                ("thread", "threadedComment", "../extra/thread.xml", false),
            ]),
        );
        put(
            &mut entries,
            "xl/extra/legacy.xml",
            t("office", "legacy-comments", &[S]),
        );
        put(
            &mut entries,
            "xl/extra/thread.xml",
            t("office", "threaded-comments", &[]),
        );
        put(
            &mut entries,
            "xl/drawings/shapes.vml",
            t("office", "vml", &[]),
        );
    }
    entries
}
fn shape(text: &str, kind: &str, n: usize) -> String {
    t(
        "office",
        "shape",
        &[&n.to_string(), &n.to_string(), kind, &xml(text)],
    )
}
fn office_presentation(note: &str, body: &str) -> Entries {
    let mut entries = base(
        "ppt/presentation.xml",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    );
    put(
        &mut entries,
        "ppt/presentation.xml",
        t("office", "presentation", &[PPT, R]),
    );
    put(
        &mut entries,
        "ppt/_rels/presentation.xml.rels",
        rels(&[("slide", "slide", "slides/custom.xml", false)]),
    );
    put(
        &mut entries,
        "ppt/slides/custom.xml",
        t("office", "slide", &[PPT, A, &shape(body, "body", 1)]),
    );
    put(
        &mut entries,
        "ppt/slides/_rels/custom.xml.rels",
        rels(&[
            ("note", "notesSlide", "../extra/odd-notes.xml", false),
            ("again", "notesSlide", "../extra/odd-notes.xml", false),
        ]),
    );
    let ignored: String = ["dt", "ftr", "hdr", "sldNum", "sldImg"]
        .into_iter()
        .enumerate()
        .map(|(i, k)| shape("IGNORED_SIGNAL", k, i + 2))
        .collect();
    put(
        &mut entries,
        "ppt/extra/odd-notes.xml",
        t("office", "notes-start", &[PPT, A, &shape(note, "body", 1)])
            + &ignored
            + "</p:spTree></p:cSld></p:notes>",
    );
    put(
        &mut entries,
        "ppt/extra/orphan.xml",
        t(
            "office",
            "orphan-notes",
            &[PPT, A, &shape("ORPHAN_SIGNAL", "body", 1)],
        ),
    );
    entries
}
fn word() -> Entries {
    let mut entries = base(
        "word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
    );
    let p = |text: &str| t("office", "paragraph", &[&xml(text)]);
    let section = t("office", "section", &[]);
    put(
        &mut entries,
        "word/document.xml",
        t(
            "office",
            "word-document",
            &[W, R, &p("BODY_SIGNAL TAB_01"), &section, &section],
        ),
    );
    put(
        &mut entries,
        "word/_rels/document.xml.rels",
        rels(&[
            ("header", "header", "extra/shared-header.xml", false),
            ("footer", "footer", "extra/shared-footer.xml", false),
            ("comment", "comments", "extra/comment.xml", false),
        ]),
    );
    put(
        &mut entries,
        "word/extra/shared-header.xml",
        t(
            "office",
            "word-header",
            &[W, &p("HEADER_SIGNAL"), &p("HEADER_TABLE_SIGNAL")],
        ),
    );
    put(
        &mut entries,
        "word/extra/shared-footer.xml",
        t("office", "word-footer", &[W, &p("FOOTER_SIGNAL")]),
    );
    put(
        &mut entries,
        "word/extra/comment.xml",
        t("office", "word-comments", &[W, &p("WORD_COMMENT_SIGNAL")]),
    );
    entries
}
pub fn office(output: &Path) -> Result<()> {
    let rows = t(
        "office",
        "formula-rows",
        &[&c("A1", "TAB_01"), &c("B1", "TAB_010")],
    );
    package(
        &output.join("search/data.xlsx"),
        office_workbook(&rows, true, false),
        true,
        false,
    )?;
    package(
        &output.join("macros/data.xlsm"),
        office_workbook(&rows, true, true),
        true,
        false,
    )?;
    package(
        &output.join("search/slides.pptx"),
        office_presentation("NOTE_SIGNAL TAB_01", "BODY_SIGNAL TAB_010"),
        true,
        false,
    )?;
    package(&output.join("search/document.docx"), word(), true, false)?;
    for fault in [
        "malformed",
        "missing",
        "external",
        "invalid-utf8",
        "oversized",
    ] {
        let mut entries = office_presentation("NOTE_SIGNAL TAB_01", "BODY_SIGNAL TAB_010");
        match fault {
            "missing" => entries.retain(|e| e.0 != "ppt/extra/odd-notes.xml"),
            "external" => put(
                &mut entries,
                "ppt/slides/_rels/custom.xml.rels",
                rels(&[(
                    "note",
                    "notesSlide",
                    "https://example.invalid/private.xml",
                    true,
                )]),
            ),
            "oversized" => put(
                &mut entries,
                "ppt/extra/odd-notes.xml",
                vec![b'x'; 32 * 1024 * 1024 + 1],
            ),
            "invalid-utf8" => put(&mut entries, "ppt/extra/odd-notes.xml", [255]),
            _ => put(&mut entries, "ppt/extra/odd-notes.xml", "<broken"),
        }
        package(
            &output.join(fault).join("slides.pptx"),
            entries,
            true,
            false,
        )?;
    }
    let mut entries = word();
    put(&mut entries, "word/extra/shared-header.xml", "<broken");
    package(
        &output.join("word-error/document.docx"),
        entries,
        true,
        false,
    )?;
    ranking(output)
}
fn condition(all: Vec<String>, any: Vec<String>) -> Value {
    json!({"mode":"conditions","scope":"file","all":all,"any":any,"not":[]})
}
fn ranking(output: &Path) -> Result<()> {
    let mut cases = vec![];
    for i in 1..=20 {
        let topic = format!("RANK_{i:02}");
        let relative = Path::new("ranking").join(format!("case-{i:02}"));
        let relative_text = relative.to_string_lossy().into_owned();
        let folder = output.join(&relative);
        fs::create_dir_all(&folder)?;
        let mut case = json!({"name":format!("rank-{i:02}"),"folder":relative_text,"query":topic,"extensions":"txt,xlsx,pptx","flags":[],"prefer":"z-good.txt","over":"a-other.txt"});
        let category = (i - 1) % 10;
        if i == 11 {
            case["query"] = json!("");
            case["flags"] = json!(["--fuzzy-search"]);
            case["spec"] = condition(vec![topic.clone(), "customer".into()], vec![]);
            write_text(&folder.join("z-good.txt"), &format!("{topic}\ncustomer\n"))?;
            write_text(&folder.join("a-other.txt"), &format!("{topic}\ncustmer\n"))?;
        } else {
            match category {
                0 => {
                    write_text(&folder.join("z-good.txt"), &topic)?;
                    write_text(
                        &folder.join("a-other.txt"),
                        &format!("prefix{topic}suffix\n").repeat(100),
                    )?;
                }
                1 => {
                    case["over"] = json!(format!("{topic}.txt"));
                    write_text(&folder.join("z-good.txt"), &format!("x {topic}.txt y"))?;
                    write_text(&folder.join(format!("{topic}.txt")), "unrelated")?;
                }
                2 | 3 => {
                    let other = format!("OTHER_{i:02}");
                    case["query"] = json!("");
                    case["prefer"] = json!("z-good.xlsx");
                    case["over"] = json!("a-other.xlsx");
                    case["spec"] = if category == 2 {
                        condition(vec![topic.clone(), other.clone()], vec![])
                    } else {
                        condition(vec![], vec![topic.clone(), other.clone()])
                    };
                    package(
                        &folder.join("z-good.xlsx"),
                        office_workbook(
                            &t(
                                "office",
                                "ranking-same-row",
                                &[&c("A1", &topic), &c("B1", &other)],
                            ),
                            false,
                            false,
                        ),
                        true,
                        false,
                    )?;
                    let rows = t("office", "ranking-first-row", &[&c("A1", &topic)])
                        + &if category == 2 {
                            t("office", "ranking-second-row", &[&c("A2", &other)])
                        } else {
                            String::new()
                        };
                    package(
                        &folder.join("a-other.xlsx"),
                        office_workbook(&rows, false, false),
                        true,
                        false,
                    )?;
                }
                4 => {
                    case["over"] = json!("a-other.pptx");
                    case["flags"] = json!(["--include-notes"]);
                    write_text(&folder.join("z-good.txt"), &topic)?;
                    package(
                        &folder.join("a-other.pptx"),
                        office_presentation(&topic, "unrelated"),
                        true,
                        false,
                    )?;
                }
                5 => {
                    case["prefer"] = json!("a-good.txt");
                    case["over"] = json!("z-other.txt");
                    write_text(&folder.join("a-good.txt"), &format!("{topic}\n").repeat(5))?;
                    write_text(
                        &folder.join("z-other.txt"),
                        &format!("{topic}\n").repeat(100),
                    )?;
                }
                6 | 7 | 8 => {
                    let (query, other) = match category {
                        6 => (format!("カタカナ{i}"), format!("かたかな{i}")),
                        7 => (
                            format!("customeridentifier{}", "a".repeat(i)),
                            format!("customeridentifer{}", "a".repeat(i)),
                        ),
                        _ => (format!("CustomerKey{i}"), format!("customer_key_{i}")),
                    };
                    case["query"] = json!(query);
                    case["flags"] = json!(["--fuzzy-search"]);
                    write_text(&folder.join("z-good.txt"), &query)?;
                    write_text(&folder.join("a-other.txt"), &other)?;
                }
                _ => {
                    case["prefer"] = json!("a-good.xlsx");
                    case["over"] = json!("z-other.xlsx");
                    case["firstAddress"] = json!("A2");
                    let rows = t(
                        "office",
                        "ranking-numeric-rows",
                        &[&c("A10", &topic), &c("A2", &topic)],
                    );
                    for name in ["a-good.xlsx", "z-other.xlsx"] {
                        package(
                            &folder.join(name),
                            office_workbook(&rows, false, false),
                            true,
                            false,
                        )?;
                    }
                }
            }
        }
        cases.push(case);
    }
    write_json(&output.join("ranking-cases.json"), &json!(cases))
}
