use regex::Regex;
pub const NETWORK_REGEX: Regex = Regex::new(
    r"(?x)
        (?P<protocol>https?:\/\/[^/]*|ipfs:\/|ipns:\/|ar:\/)?
        (?P<root>\/)?
        (?P<subpath>ipfs\/|ipns\/)?
        (?P<target>[\w\-.]+)
        (?P<subtarget>\/.*)?
    ",
)
.unwrap();

pub const IPFS_HASH_REGEX: Regex = Regex::new(
    r"(?x)
        ^
        (
            Qm[1-9A-HJ-NP-Za-km-z]{44,}
          | b[A-Za-z2-7]{58,}
          | B[A-Z2-7]{58,}
          | z[1-9A-HJ-NP-Za-km-z]{48,}
          | F[0-9A-F]{50,}
        )
        (\/(?P<target>[\w\-.]+))?
        (?P<subtarget>\/.*)?$
    ",
)
.unwrap();

pub const BASE64_REGEX: Regex = {
    Regex::new(
        r#"
        ^data:([a-zA-Z\-/+]*);base64,([^\"].*)
        "#,
    )
    .unwrap();

pub const DATA_URI_REGEX: Regex =
    Regex::new("^data:([a-zA-Z\\-/+]*)?(;[a-zA-Z0-9].*?)?(,)").unwrap();
