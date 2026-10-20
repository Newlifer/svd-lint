//! Minimal corpus generators, sharing the Stage 3 integration scenarios.
fn xml(properties: &str, blocks: &str, registers: &str) -> String {
    format!(
        r#"<device schemaVersion="1.3"><name>D</name><version>1</version><description>d</description><addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><access>read-write</access>{properties}<peripherals><peripheral><name>P</name><description>p</description><baseAddress>0x1000</baseAddress>{blocks}<registers>{registers}</registers></peripheral></peripherals></device>"#
    )
}
fn reg(name: &str, offset: u32, size: u32, extra: &str, fields: &str) -> String {
    let fields = if fields.is_empty() {
        String::new()
    } else {
        format!("<fields>{fields}</fields>")
    };
    format!(
        "<register><name>{name}</name><addressOffset>{offset}</addressOffset><size>{size}</size>{extra}{fields}</register>"
    )
}
fn field(name: &str, offset: u32, width: u32, extra: &str) -> String {
    format!(
        "<field><name>{name}</name><bitOffset>{offset}</bitOffset><bitWidth>{width}</bitWidth>{extra}</field>"
    )
}
fn one(size: u32, extra: &str, fields: &str) -> String {
    xml("", "", &reg("R", 0, size, extra, fields))
}
fn entries(items: &str, usage: &str) -> String {
    let usage = if usage.is_empty() {
        String::new()
    } else {
        format!("<usage>{usage}</usage>")
    };
    format!("<enumeratedValues>{usage}{items}</enumeratedValues>")
}
fn entry(name: &str, value: &str) -> String {
    format!("<enumeratedValue><name>{name}</name><value>{value}</value></enumeratedValue>")
}
fn enum_doc(width: u32, items: &str) -> String {
    one(width, "", &field("F", 0, width, &entries(items, "")))
}
fn block(offset: u32, size: u32) -> String {
    format!(
        "<addressBlock><offset>{offset}</offset><size>{size}</size><usage>registers</usage></addressBlock>"
    )
}

macro_rules! cases {
    ($pass:ident,$fail:ident,$boundary:ident,$code:literal,$severity:expr,$valid:expr,$invalid:expr,$edge:expr) => {
        fn $pass() -> (&'static str, String, String, String) {
            ($code, $valid, $invalid, $edge)
        }
    };
}

cases!(
    sem001_valid,
    sem001_invalid,
    sem001_boundary,
    "E018",
    Severity::Error,
    one(32, "", ""),
    one(0, "", ""),
    one(u32::MAX, "", "")
);
cases!(
    sem002_valid,
    sem002_invalid,
    sem002_boundary,
    "E019",
    Severity::Error,
    one(8, "", &field("F", 0, 1, "")),
    one(8, "", &field("F", 0, 0, "")),
    one(u32::MAX, "", &field("F", 0, u32::MAX, ""))
);
cases!(
    sem003_valid,
    sem003_invalid,
    sem003_boundary,
    "E020",
    Severity::Error,
    one(8, "", &field("F", 0, 2, "")),
    one(8, "", &field("F", 7, 2, "")),
    one(8, "", &field("F", 7, 1, ""))
);
cases!(
    sem004_valid,
    sem004_invalid,
    sem004_boundary,
    "W003",
    Severity::Warning,
    one(8, "", &(field("A", 0, 1, "") + &field("B", 1, 1, ""))),
    one(8, "", &(field("A", 0, 4, "") + &field("B", 3, 2, ""))),
    one(8, "", &(field("A", 0, 4, "") + &field("B", 4, 4, "")))
);
cases!(
    sem005_valid,
    sem005_invalid,
    sem005_boundary,
    "E021",
    Severity::Error,
    one(8, "<resetValue>255</resetValue>", ""),
    one(8, "<resetValue>256</resetValue>", ""),
    one(64, "<resetValue>0xffffffffffffffff</resetValue>", "")
);
cases!(
    sem006_valid,
    sem006_invalid,
    sem006_boundary,
    "E022",
    Severity::Error,
    one(8, "<resetMask>255</resetMask>", ""),
    one(8, "<resetMask>256</resetMask>", ""),
    one(64, "<resetMask>0xffffffffffffffff</resetMask>", "")
);
cases!(
    sem007_valid,
    sem007_invalid,
    sem007_boundary,
    "W004",
    Severity::Warning,
    one(8, "<resetValue>1</resetValue><resetMask>1</resetMask>", ""),
    one(8, "<resetValue>2</resetValue><resetMask>1</resetMask>", ""),
    one(8, "<resetValue>0</resetValue><resetMask>0</resetMask>", "")
);
cases!(
    sem008_valid,
    sem008_invalid,
    sem008_boundary,
    "E023",
    Severity::Error,
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "") + &reg("B", 2, 16, "", ""))
    ),
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "") + &reg("B", 1, 16, "", ""))
    ),
    xml("", "", &(reg("A", 0, 9, "", "") + &reg("B", 2, 1, "", "")))
);
cases!(
    sem009_valid,
    sem009_invalid,
    sem009_boundary,
    "E024",
    Severity::Error,
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "")
            + &reg("B", 0, 16, "<alternateRegister>A</alternateRegister>", ""))
    ),
    one(16, "<alternateRegister>MISSING</alternateRegister>", ""),
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "")
            + &reg("B", 0, 16, "<alternateRegister>A</alternateRegister>", "")
            + &reg("C", 0, 16, "<alternateRegister>B</alternateRegister>", ""))
    )
);
cases!(
    sem010_valid,
    sem010_invalid,
    sem010_boundary,
    "E025",
    Severity::Error,
    xml("", &block(0, 8), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 8), &reg("R", 8, 8, "", "")),
    xml("", &block(0, 8), &reg("R", 7, 8, "", ""))
);
cases!(
    sem011_valid,
    sem011_invalid,
    sem011_boundary,
    "E026",
    Severity::Error,
    xml("", &block(0, 4), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 0), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 1), &reg("R", 0, 8, "", "")).replace(
        "<baseAddress>0x1000</baseAddress>",
        "<baseAddress>0xffffffffffffffff</baseAddress>"
    )
);
cases!(
    sem012_valid,
    sem012_invalid,
    sem012_boundary,
    "E027",
    Severity::Error,
    enum_doc(2, &entry("A", "3")),
    enum_doc(2, &entry("A", "4")),
    enum_doc(64, &entry("A", "0xffffffffffffffff"))
);
cases!(
    sem013_valid,
    sem013_invalid,
    sem013_boundary,
    "E028",
    Severity::Error,
    enum_doc(2, &(entry("A", "0") + &entry("B", "1"))),
    enum_doc(2, &(entry("A", "0") + &entry("A", "1"))),
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            &(entries(&entry("A", "0"), "read") + &entries(&entry("B", "0"), "write"))
        )
    )
);
cases!(
    sem014_valid,
    sem014_invalid,
    sem014_boundary,
    "E029",
    Severity::Error,
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><isDefault>true</isDefault></enumeratedValue>"
    ),
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><isDefault>true</isDefault></enumeratedValue><enumeratedValue><name>E</name><isDefault>true</isDefault></enumeratedValue>"
    ),
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><value>255</value><isDefault>true</isDefault></enumeratedValue>"
    )
);
cases!(
    sem015_valid,
    sem015_invalid,
    sem015_boundary,
    "E030",
    Severity::Error,
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            "<writeConstraint><range><minimum>0</minimum><maximum>3</maximum></range></writeConstraint>"
        )
    ),
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            "<writeConstraint><range><minimum>3</minimum><maximum>2</maximum></range></writeConstraint>"
        )
    ),
    one(
        64,
        "",
        &field(
            "F",
            0,
            64,
            "<writeConstraint><range><minimum>0</minimum><maximum>0xffffffffffffffff</maximum></range></writeConstraint>"
        )
    )
);
cases!(
    sem016_valid,
    sem016_invalid,
    sem016_boundary,
    "W005",
    Severity::Warning,
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>read-only</access><readAction>clear</readAction>"
        )
    ),
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>write-only</access><readAction>clear</readAction>"
        )
    ),
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>read-writeOnce</access><readAction>clear</readAction><modifiedWriteValues>oneToClear</modifiedWriteValues>"
        )
    )
);
pub fn semantic_cases() -> Vec<(&'static str, String, String, String)> {
    vec![
        sem001_valid(),
        sem002_valid(),
        sem003_valid(),
        sem004_valid(),
        sem005_valid(),
        sem006_valid(),
        sem007_valid(),
        sem008_valid(),
        sem009_valid(),
        sem010_valid(),
        sem011_valid(),
        sem012_valid(),
        sem013_valid(),
        sem014_valid(),
        sem015_valid(),
        sem016_valid(),
        (
            "W006",
            xml("", "", &(reg("A", 0, 8, "", "") + &reg("B", 1, 8, "", ""))),
            xml("", "", &(reg("A", 0, 8, "", "") + &reg("B", 0, 8, "", ""))),
            xml(
                "",
                "",
                &(reg("A", 0, 8, "", "")
                    + &reg("B", 0, 8, "<alternateRegister>A</alternateRegister>", "")),
            ),
        ),
        (
            "W007",
            enum_doc(2, &(entry("A", "0") + &entry("B", "1"))),
            enum_doc(2, &(entry("A", "0") + &entry("B", "0"))),
            one(
                2,
                "",
                &field(
                    "F",
                    0,
                    2,
                    &(entries(&entry("A", "0"), "read") + &entries(&entry("B", "0"), "write")),
                ),
            ),
        ),
    ]
}

pub fn normalization_cases() -> Vec<(&'static str, String)> {
    let array = |dim: u32, index: &str, name: &str| {
        reg(name, 0, 32, "", "").replace(
            "<register>",
            &format!("<register><dim>{dim}</dim><dimIncrement>4</dimIncrement>{index}"),
        )
    };
    vec![
        (
            "E010",
            xml(
                "",
                "",
                &reg("R", 0, 32, "", "").replace("<register>", "<register derivedFrom=\"R\">"),
            ),
        ),
        (
            "E011",
            xml(
                "",
                "",
                &(reg("R", 0, 32, "", "") + &reg("R", 4, 32, "", "")),
            ),
        ),
        ("E012", xml("", "", &array(0, "", "R%s"))),
        (
            "E013",
            xml("", "", &array(2, "<dimIndex>A, A</dimIndex>", "R%s")),
        ),
        ("E014", xml("", "", &array(2, "", "R%s%s"))),
        (
            "E015",
            xml("", "", &reg("R", 1, 32, "", "")).replace(
                "<baseAddress>0x1000</baseAddress>",
                "<baseAddress>0xffffffffffffffff</baseAddress>",
            ),
        ),
        (
            "E016",
            xml("", "", &reg("R", 0, 32, "", "")).replace("<size>32</size>", ""),
        ),
        ("E017", xml("", "", &array(100_001, "", "R%s"))),
    ]
}
pub fn regression_whitespace() -> String {
    let array = reg("R%s", 0, 32, "", "").replace(
        "<register>",
        "<register><dim>3</dim><dimIncrement>4</dimIncrement><dimIndex>F,G, H</dimIndex>",
    );
    let derived =
        reg("COPY", 12, 32, "", "").replace("<register>", "<register derivedFrom=\"RH\">");
    xml("", "", &(array + &derived))
}
