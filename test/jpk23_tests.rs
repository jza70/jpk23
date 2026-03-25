use super::*;
use std::io::Cursor;

#[test]
fn test_valid_root_namespace_conversion() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/" xmlns:etd="http://crd.gov.pl/xml/schematy/dziedzinowe/mf/2021/06/08/eD/DefinicjeTypy/"></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains(r#"xmlns="http://crd.gov.pl/wzor/2025/12/19/14090/""#));
    assert!(result.contains(r#"xmlns:etd="http://crd.gov.pl/xml/schematy/dziedzinowe/mf/2022/09/13/eD/DefinicjeTypy/""#));
}

#[test]
fn test_invalid_root_namespace_fails() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><JPK xmlns="http://crd.gov.pl/wzor/WRONG/"></JPK>"#;
    let mut output = Vec::new();
    let result = process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Error: Input file must be of JPK_V7 (1 or 2) type root namespace.");
}

#[test]
fn test_sales_row_di_injection_wew() {
    let input = br#"<?xml version="1.0"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><SprzedazWiersz><DataWystawienia>2026</DataWystawienia><TypDokumentu>WEW</TypDokumentu></SprzedazWiersz></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    // Ensure <DI>1</DI> was injected correctly for WEW
    assert!(result.contains("<DataWystawienia>2026</DataWystawienia><DI>1</DI><TypDokumentu>WEW</TypDokumentu>"));
}

#[test]
fn test_sales_row_bfk_injection_normal_invoice() {
    let input = br#"<?xml version="1.0"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><SprzedazWiersz><DataSprzedazy>2026</DataSprzedazy><TypDokumentu>VAT</TypDokumentu></SprzedazWiersz></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    // Ensure <BFK>1</BFK> was injected for a standard (non-WEW, non-RO) invoice right after Date
    assert!(result.contains("<DataSprzedazy>2026</DataSprzedazy><BFK>1</BFK><TypDokumentu>VAT</TypDokumentu>"));
}

#[test]
fn test_purchase_row_di_injection() {
    let input = br#"<?xml version="1.0"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><ZakupWiersz><DataZakupu>2026</DataZakupu><DokumentZakupu>WEW</DokumentZakupu></ZakupWiersz></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains("<DataZakupu>2026</DataZakupu><DI>1</DI><DokumentZakupu>WEW</DokumentZakupu>"));
}

#[test]
fn test_namespace_override() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><Naglowek><KodFormularza kodSystemowy="JPK_V7M (2)">JPK_V7M</KodFormularza></Naglowek></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, Some("foo".to_string()), None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains("<foo:JPK"));
    assert!(result.contains(r#"xmlns:foo="http://crd.gov.pl/wzor/2025/12/19/14090/""#));
    assert!(result.contains("<foo:Naglowek>"));
    assert!(result.contains(r#"<foo:KodFormularza kodSystemowy="JPK_V7M (3)""#));
}

#[test]
fn test_namespace_strip() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><ext:JPK xmlns:ext="http://crd.gov.pl/wzor/2021/12/27/11148/"><ext:Naglowek><ext:KodFormularza kodSystemowy="JPK_V7M (2)">JPK_V7M</ext:KodFormularza></ext:Naglowek></ext:JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, Some("".to_string()), None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains("<JPK xmlns="));
    assert!(result.contains(r#"xmlns="http://crd.gov.pl/wzor/2025/12/19/14090/""#));
    assert!(result.contains("<Naglowek>"));
    assert!(result.contains(r#"<KodFormularza kodSystemowy="JPK_V7M (3)""#));
    assert!(!result.contains("ext:"));
}

#[test]
fn test_control_total_discrepancy() {
    let input = br#"<?xml version="1.0"?>
    <JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/">
        <ZakupWiersz><K_43>100.00</K_43><K_44>23.00</K_44></ZakupWiersz>
        <ZakupWiersz><K_43>200.00</K_43><K_44>46.00</K_44></ZakupWiersz>
        <ZakupCtrl>
            <LiczbaWierszyZakupow>1</LiczbaWierszyZakupow>
            <PodatekNaliczony>50.00</PodatekNaliczony>
        </ZakupCtrl>
    </JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    // Sum of K_43 + K_44 is 300 + 69 = 369.00
    // But we should keep the input's 50.00 and add a comment.
    assert!(result.contains("<LiczbaWierszyZakupow>2</LiczbaWierszyZakupow>"));
    assert!(result.contains("<PodatekNaliczony>50.00</PodatekNaliczony>"));
    assert!(result.contains("<!-- Warning: Control total discrepancy in ZakupCtrl/PodatekNaliczony is 50.00 (calculated 369.00) -->"));
}


#[test]
fn test_variant_override_k() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><Naglowek><KodFormularza>JPK_V7M</KodFormularza></Naglowek></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::K, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains(r#"xmlns="http://crd.gov.pl/wzor/2025/12/19/14089/""#));
    assert!(result.contains(r#"xsi:schemaLocation="http://crd.gov.pl/wzor/2025/12/19/14089/ JPK_V7K3.xsd""#));
    assert!(result.contains(r#"kodSystemowy="JPK_V7K (3)""#));
    assert!(result.contains(r#"wersjaSchemy="1-0E""#));
    assert!(result.contains("<WariantFormularza>3</WariantFormularza>"));
}

#[test]
fn test_v2_k_detection() {
    let input = br#"<?xml version="1.0" encoding="UTF-8"?><JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11149/"><Naglowek><KodFormularza>JPK_V7K</KodFormularza></Naglowek></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::Unknown, false).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains(r#"xmlns="http://crd.gov.pl/wzor/2025/12/19/14089/""#));
    assert!(result.contains(r#"xsi:schemaLocation="http://crd.gov.pl/wzor/2025/12/19/14089/ JPK_V7K3.xsd""#));
    assert!(result.contains(r#"kodSystemowy="JPK_V7K (3)""#));
}

#[test]
fn test_xml_indentation() {
    let input = br#"<JPK xmlns="http://crd.gov.pl/wzor/2021/12/27/11148/"><Naglowek><KodFormularza>JPK_V7M</KodFormularza></Naglowek></JPK>"#;
    let mut output = Vec::new();
    process_jpk(Cursor::new(input), &mut output, None, None, FormVariant::M, true).unwrap();
    let result = String::from_utf8(output).unwrap();
    
    assert!(result.contains("\n  <Naglowek>"));
    assert!(result.contains("\n    <KodFormularza"));
}

