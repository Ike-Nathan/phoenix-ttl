//! Compact, human-readable rendering of storage keys and addresses for reports.

use soroban_sdk::xdr::{ScAddress, ScVal};

/// Shortens a strkey to `CAAA…ITA4`; short strings are returned unchanged.
pub fn short_address(strkey: &str) -> String {
    if strkey.len() < 20 {
        return strkey.to_string();
    }
    format!("{}…{}", &strkey[..4], &strkey[strkey.len() - 4..])
}

pub(crate) fn address(a: &ScAddress) -> String {
    a.to_string()
}

/// Renders a storage key. Vectors that start with a symbol are shown as enum
/// variants (`Shares(CAAA…K3IM)`), which is how `#[contracttype]` enums are
/// encoded.
pub fn key(v: &ScVal) -> String {
    render(v, true)
}

/// Renders any value without the enum heuristic.
pub fn value(v: &ScVal) -> String {
    render(v, false)
}

fn render(v: &ScVal, enum_style: bool) -> String {
    match v {
        ScVal::Bool(b) => b.to_string(),
        ScVal::Void => "()".into(),
        ScVal::U32(n) => format!("{n}u32"),
        ScVal::I32(n) => format!("{n}i32"),
        ScVal::U64(n) => format!("{n}u64"),
        ScVal::I64(n) => format!("{n}i64"),
        ScVal::Timepoint(t) => format!("{}tp", t.0),
        ScVal::Duration(d) => format!("{}dur", d.0),
        ScVal::U128(p) => format!("{}u128", (u128::from(p.hi) << 64) | u128::from(p.lo)),
        ScVal::I128(p) => format!("{}i128", (i128::from(p.hi) << 64) | i128::from(p.lo)),
        ScVal::Symbol(s) => String::from_utf8_lossy(s.0.as_slice()).into_owned(),
        ScVal::String(s) => format!("{:?}", String::from_utf8_lossy(s.0.as_slice())),
        ScVal::Bytes(b) => format!("0x{}", hex(b.0.as_slice())),
        ScVal::Address(a) => short_address(&address(a)),
        ScVal::Vec(Some(items)) => {
            let items: Vec<&ScVal> = items.0.iter().collect();
            if enum_style {
                if let Some(ScVal::Symbol(name)) = items.first() {
                    let name = String::from_utf8_lossy(name.0.as_slice()).into_owned();
                    if items.len() == 1 {
                        return name;
                    }
                    let args: Vec<String> = items[1..].iter().map(|x| render(x, false)).collect();
                    return format!("{name}({})", args.join(", "));
                }
            }
            let parts: Vec<String> = items.iter().map(|x| render(x, false)).collect();
            format!("[{}]", parts.join(", "))
        }
        ScVal::Vec(None) => "[]".into(),
        ScVal::Map(Some(m)) => {
            let parts: Vec<String> =
                m.0.iter()
                    .map(|e| format!("{}: {}", render(&e.key, false), render(&e.val, false)))
                    .collect();
            format!("{{{}}}", parts.join(", "))
        }
        ScVal::Map(None) => "{}".into(),
        ScVal::LedgerKeyContractInstance => "contract instance".into(),
        ScVal::LedgerKeyNonce(n) => format!("nonce({})", n.nonce),
        // 256-bit integers, errors, instances and anything added in later
        // protocols: fall back to the XDR debug form rather than guessing.
        other => format!("{other:?}"),
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::xdr::{Int128Parts, ScSymbol, ScVec, StringM};

    fn sym(s: &str) -> ScVal {
        ScVal::Symbol(ScSymbol(StringM::try_from(s).unwrap()))
    }

    #[test]
    fn renders_enum_keys() {
        let v = ScVal::Vec(Some(ScVec(
            vec![sym("Shares"), ScVal::U32(7)].try_into().unwrap(),
        )));
        assert_eq!(key(&v), "Shares(7u32)");
        assert_eq!(value(&v), "[Shares, 7u32]");
        let unit = ScVal::Vec(Some(ScVec(vec![sym("Admin")].try_into().unwrap())));
        assert_eq!(key(&unit), "Admin");
    }

    #[test]
    fn renders_numbers() {
        let neg = ScVal::I128(Int128Parts {
            hi: -1,
            lo: u64::MAX,
        });
        assert_eq!(value(&neg), "-1i128");
        let big = ScVal::I128(Int128Parts { hi: 1, lo: 0 });
        assert_eq!(value(&big), "18446744073709551616i128");
        assert_eq!(value(&ScVal::Void), "()");
    }

    #[test]
    fn shortens_addresses() {
        assert_eq!(
            short_address("CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAITA4"),
            "CAAA…ITA4"
        );
        assert_eq!(short_address("short"), "short");
    }
}
