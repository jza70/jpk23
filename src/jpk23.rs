use anyhow::{Result, bail};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::{BufRead, Write};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FormVariant {
    Unknown,
    M,
    K,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FormVersion {
    Unknown,
    V1,
    V2,
    V3,
}

#[derive(Debug)]
pub struct RateBreakdown {
    pub base_5: f64,
    pub vat_5: f64,
    pub base_8: f64,
    pub vat_8: f64,
    pub base_23: f64,
    pub vat_23: f64,
    pub base_other: f64,
    pub vat_other: f64,
}

impl RateBreakdown {
    pub fn new() -> Self {
        Self {
            base_5: 0.0, vat_5: 0.0,
            base_8: 0.0, vat_8: 0.0,
            base_23: 0.0, vat_23: 0.0,
            base_other: 0.0, vat_other: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct JpkStats {
    pub original_version: FormVersion,
    pub taxpayer_nip: Option<String>,
    
    pub sales_count: usize,
    pub total_sales_base: f64,
    pub total_sales_vat: f64,
    pub max_sales_vat: Option<(f64, usize)>,
    pub min_sales_vat: Option<(f64, usize)>,
    pub breakdown: RateBreakdown,
    
    pub purchase_count: usize,
    pub total_purchase_base: f64,
    pub total_purchase_vat: f64,
    pub max_purchase_vat: Option<(f64, usize)>,
    pub min_purchase_vat: Option<(f64, usize)>,
}

struct ParserState {
    version: FormVersion,
    variant: FormVariant,
    has_checked_root: bool,
    root_prefix: Vec<u8>,
    target_prefix: Option<Vec<u8>>,
    
    inside_naglowek: bool,
    naglowek_buffer: Vec<Event<'static>>,
    
    inside_sales: bool,
    sales_buffer: Vec<Event<'static>>,
    
    inside_purchase: bool,
    purchase_buffer: Vec<Event<'static>>,
    
    inside_podmiot: bool,
    podmiot_buffer: Vec<Event<'static>>,
    
    has_started_ewidencja: bool,
    kod_urzedu: Option<String>,
    explicit_variant: FormVariant,
    
    // Control total accumulators
    sum_k16: f64, sum_k18: f64, sum_k20: f64, sum_k24: f64, sum_k26: f64, sum_k28: f64,
    sum_k30: f64, sum_k32: f64, sum_k33: f64, sum_k34: f64,
    sum_k35: f64, sum_k36: f64, sum_k360: f64,
    
    sum_k41: f64, sum_k43: f64, sum_k44: f64, sum_k45: f64, sum_k46: f64, sum_k47: f64,
    
    taxpayer_nip: Option<String>,
    
    count_sales: usize,
    count_purchase: usize,
    
    total_sales_base: f64,
    total_sales_vat: f64,
    total_purchase_base: f64,
    total_purchase_vat: f64,
    
    breakdown: RateBreakdown,
    
    max_sales_vat: Option<(f64, usize)>,
    min_sales_vat: Option<(f64, usize)>,
    max_purchase_vat: Option<(f64, usize)>,
    min_purchase_vat: Option<(f64, usize)>,
    
    // Helper to distinguish if we are in a control section
    inside_ctrl: bool,
    ctrl_buffer: Vec<Event<'static>>,
}

impl ParserState {
    fn new(target_namespace: Option<String>, kod_urzedu: Option<String>, explicit_variant: FormVariant) -> Self {
        Self {
            version: FormVersion::Unknown,
            variant: FormVariant::Unknown,
            has_checked_root: false,
            root_prefix: Vec::new(),
            target_prefix: target_namespace.map(|s| s.into_bytes()),
            
            inside_naglowek: false,
            naglowek_buffer: Vec::new(),
            inside_sales: false,
            sales_buffer: Vec::new(),
            inside_purchase: false,
            purchase_buffer: Vec::new(),
            inside_podmiot: false,
            podmiot_buffer: Vec::new(),
            
            has_started_ewidencja: false,
            kod_urzedu,
            explicit_variant,
            
            sum_k16: 0.0, sum_k18: 0.0, sum_k20: 0.0, sum_k24: 0.0, sum_k26: 0.0, sum_k28: 0.0,
            sum_k30: 0.0, sum_k32: 0.0, sum_k33: 0.0, sum_k34: 0.0,
            sum_k35: 0.0, sum_k36: 0.0, sum_k360: 0.0,
            
            sum_k41: 0.0, sum_k43: 0.0, sum_k44: 0.0, sum_k45: 0.0, sum_k46: 0.0, sum_k47: 0.0,
            
            taxpayer_nip: None,
            
            count_sales: 0,
            count_purchase: 0,
            
            total_sales_base: 0.0,
            total_sales_vat: 0.0,
            total_purchase_base: 0.0,
            total_purchase_vat: 0.0,
            
            breakdown: RateBreakdown::new(),
            
            max_sales_vat: None,
            min_sales_vat: None,
            max_purchase_vat: None,
            min_purchase_vat: None,
            
            inside_ctrl: false,
            ctrl_buffer: Vec::new(),
        }
    }
}

pub fn process_jpk<R: BufRead, W: Write>(
    input: R,
    writer: &mut Writer<W>,
    target_namespace: Option<String>,
    kod_urzedu: Option<String>,
    explicit_variant: FormVariant,
) -> Result<JpkStats> {
    let mut reader = Reader::from_reader(input);
    let mut buf = Vec::new();
    let mut state = ParserState::new(target_namespace, kod_urzedu, explicit_variant);

    loop {
        let event = reader.read_event_into(&mut buf)?;
        match event {
            Event::Eof => break,
            Event::Start(ref e) => handle_start_event(e, &mut state, writer)?,
            Event::Text(ref text) => handle_text_event(text, &mut state, writer)?,
            Event::End(ref e) => handle_end_event(e, &mut state, writer)?,
            Event::Empty(ref e) => handle_empty_event(e, &mut state, writer)?,
            other => handle_other_event(other, &mut state, writer)?,
        }
    }
    
    if !state.has_checked_root {
        bail!("Error: Unrecognized format or empty file, could not find JPK root node.");
    }

    Ok(JpkStats {
        original_version: state.version,
        taxpayer_nip: state.taxpayer_nip,
        sales_count: state.count_sales,
        total_sales_base: state.total_sales_base,
        total_sales_vat: state.total_sales_vat,
        max_sales_vat: state.max_sales_vat,
        min_sales_vat: state.min_sales_vat,
        breakdown: state.breakdown,
        purchase_count: state.count_purchase,
        total_purchase_base: state.total_purchase_base,
        total_purchase_vat: state.total_purchase_vat,
        max_purchase_vat: state.max_purchase_vat,
        min_purchase_vat: state.min_purchase_vat,
    })
}

fn split_name(name: &[u8]) -> (&[u8], &[u8]) {
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        (&name[..pos], &name[pos + 1..])
    } else {
        (&[], name)
    }
}

fn build_tag(prefix: &[u8], local: &str) -> String {
    if prefix.is_empty() {
        local.to_string()
    } else {
        format!("{}:{}", String::from_utf8_lossy(prefix), local)
    }
}

fn write_start<W: Write>(writer: &mut Writer<W>, e: &BytesStart, state: &ParserState) -> Result<()> {
    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    let (_, local_name) = split_name(e.name().into_inner());
    let new_name = build_tag(active_prefix, std::str::from_utf8(local_name).unwrap_or(""));
    let mut new_e = BytesStart::new(new_name);
    for attr in e.attributes().flatten() {
        let key_str = std::str::from_utf8(attr.key.into_inner()).unwrap_or("");
        let val_str = std::str::from_utf8(attr.value.as_ref()).unwrap_or("");
        new_e.push_attribute((key_str, val_str));
    }
    writer.write_event(Event::Start(new_e))?;
    Ok(())
}

fn write_empty<W: Write>(writer: &mut Writer<W>, e: &BytesStart, state: &ParserState) -> Result<()> {
    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    let (_, local_name) = split_name(e.name().into_inner());
    let new_name = build_tag(active_prefix, std::str::from_utf8(local_name).unwrap_or(""));
    let mut new_e = BytesStart::new(new_name);
    for attr in e.attributes().flatten() {
        let key_str = std::str::from_utf8(attr.key.into_inner()).unwrap_or("");
        let val_str = std::str::from_utf8(attr.value.as_ref()).unwrap_or("");
        new_e.push_attribute((key_str, val_str));
    }
    writer.write_event(Event::Empty(new_e))?;
    Ok(())
}

fn write_end<W: Write>(writer: &mut Writer<W>, e: &BytesEnd, state: &ParserState) -> Result<()> {
    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    let (_, local_name) = split_name(e.name().into_inner());
    let new_name = build_tag(active_prefix, std::str::from_utf8(local_name).unwrap_or(""));
    writer.write_event(Event::End(BytesEnd::new(new_name)))?;
    Ok(())
}

fn group_children(buffer: &[Event<'static>]) -> Vec<(String, Vec<Event<'static>>)> {
    let mut children = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();
    let mut depth = 0;

    for ev in buffer {
        match ev {
            Event::Start(e) => {
                if depth == 1 {
                    let (_, local) = split_name(e.name().into_inner());
                    current_name = String::from_utf8_lossy(local).to_string();
                }
                depth += 1;
                if depth > 1 {
                    current_block.push(ev.clone());
                }
            }
            Event::Empty(e) => {
                if depth == 1 {
                    let (_, local) = split_name(e.name().into_inner());
                    current_name = String::from_utf8_lossy(local).to_string();
                    current_block.push(ev.clone());
                    children.push((current_name.clone(), current_block.clone()));
                    current_block.clear();
                    current_name.clear();
                } else if depth > 1 {
                    current_block.push(ev.clone());
                }
            }
            Event::End(_e) => {
                depth -= 1;
                if depth == 1 {
                    current_block.push(ev.clone());
                    children.push((current_name.clone(), current_block.clone()));
                    current_block.clear();
                    current_name.clear();
                } else if depth > 1 {
                    current_block.push(ev.clone());
                }
            }
            other => {
                if depth > 1 {
                    current_block.push(other.clone());
                }
            }
        }
    }
    children
}

fn handle_start_event<W: Write>(
    e: &BytesStart,
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    let full_name = e.name().into_inner();
    let (prefix, local_name) = split_name(full_name);
    
    if !state.has_checked_root && local_name == b"JPK" {
        state.has_checked_root = true;
        state.root_prefix = prefix.to_vec();
        
        // If explicit variant was provided via CLI, use it immediately
        if state.explicit_variant != FormVariant::Unknown {
            state.variant = state.explicit_variant;
        }

        for attr in e.attributes().flatten() {
            let k = attr.key.into_inner();
            let is_default = k == b"xmlns";
            let is_prefix = !prefix.is_empty() && k.starts_with(b"xmlns:") && &k[6..] == prefix;
            if is_default || is_prefix {
                let val = attr.value.as_ref();
                if val == b"http://crd.gov.pl/wzor/2021/12/27/11148/" {
                    state.version = FormVersion::V2;
                    if state.variant == FormVariant::Unknown { state.variant = FormVariant::M; }
                } else if val == b"http://crd.gov.pl/wzor/2021/12/27/11149/" {
                    state.version = FormVersion::V2;
                    if state.variant == FormVariant::Unknown { state.variant = FormVariant::K; }
                } else if val == b"http://jpk.mf.gov.pl/wzor/2017/11/13/1113/" {
                    state.version = FormVersion::V1;
                } else if val == b"http://crd.gov.pl/wzor/2025/12/19/14090/" {
                    state.version = FormVersion::V3;
                    if state.variant == FormVariant::Unknown { state.variant = FormVariant::M; }
                } else if val == b"http://crd.gov.pl/wzor/2025/12/19/14089/" {
                    state.version = FormVersion::V3;
                    if state.variant == FormVariant::Unknown { state.variant = FormVariant::K; }
                }
            }
        }
        if state.version == FormVersion::Unknown {
            bail!("Error: Unrecognized format or namespace. Please provide a supported JPK_V7 XML file (V1, V2 or V3).");
        }
        
        // If still unknown (V1 or weird V2), default to M for now (will be updated in Naglowek if V1)
        // But the user says "when we convert with -k" so we should trust state.variant if set.
        if state.variant == FormVariant::Unknown {
            state.variant = FormVariant::M;
        }

        let mut e_owned = e.clone().into_owned();
        e_owned.clear_attributes();
        
        let ns_m = "http://crd.gov.pl/wzor/2025/12/19/14090/";
        let ns_k = "http://crd.gov.pl/wzor/2025/12/19/14089/";
        let (ns, schema) = if state.variant == FormVariant::K {
            (ns_k, format!("{} JPK_V7K3.xsd", ns_k))
        } else {
            (ns_m, format!("{} JPK_V7M3.xsd", ns_m))
        };

        for attr in e.attributes().flatten() {
            let k = attr.key.into_inner();
            let is_default = k == b"xmlns";
            let is_prefix = !prefix.is_empty() && k.starts_with(b"xmlns:") && &k[6..] == prefix;
            
            if is_default || is_prefix {
                let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
                if active_prefix.is_empty() {
                    e_owned.push_attribute(("xmlns", ns));
                } else {
                    let p_str = std::str::from_utf8(active_prefix).unwrap_or("");
                    let key_str = format!("xmlns:{}", p_str);
                    e_owned.push_attribute((key_str.as_str(), ns));
                }
            } else if k == b"xmlns:etd" {
                e_owned.push_attribute(("xmlns:etd", "http://crd.gov.pl/xml/schematy/dziedzinowe/mf/2022/09/13/eD/DefinicjeTypy/"));
            } else if k == b"xmlns:tns" {
                // remove tns entirely as it's legacy V1
            } else if k == b"xmlns:xsi" || k == b"xsi:schemaLocation" {
                // skip to re-add correctly
            } else {
                e_owned.push_attribute(attr);
            }
        }
        
        e_owned.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
        e_owned.push_attribute(("xsi:schemaLocation", schema.as_str()));

        write_start(writer, &e_owned, state)?;
        return Ok(());
    }
    
    if local_name == b"Naglowek" {
        state.inside_naglowek = true;
        state.naglowek_buffer.clear();
        state.naglowek_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if local_name == b"SprzedazCtrl" || local_name == b"ZakupCtrl" {
        state.inside_ctrl = true;
        state.ctrl_buffer.clear();
        state.ctrl_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_ctrl {
        state.ctrl_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_naglowek {
        state.naglowek_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if local_name == b"Podmiot1" {
        state.inside_podmiot = true;
        state.podmiot_buffer.clear();
        state.podmiot_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_podmiot {
        state.podmiot_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }

    // Auto-Ewidencja for V1
    if state.version == FormVersion::V1 && !state.has_started_ewidencja {
        if local_name == b"SprzedazWiersz" || local_name == b"ZakupWiersz" || local_name == b"SprzedazCtrl" || local_name == b"ZakupCtrl" {
            let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
            let tag = build_tag(active_prefix, "Ewidencja");
            writer.write_event(Event::Start(BytesStart::new(&tag)))?;
            state.has_started_ewidencja = true;
        }
    }
    
    if local_name == b"SprzedazWiersz" && !state.inside_purchase {
        state.inside_sales = true;
        state.sales_buffer.clear();
        state.sales_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if local_name == b"ZakupWiersz" && !state.inside_sales {
        state.inside_purchase = true;
        state.purchase_buffer.clear();
        state.purchase_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_sales {
        state.sales_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }

    if state.inside_purchase {
        state.purchase_buffer.push(Event::Start(e.clone().into_owned()));
        return Ok(());
    }
    
    write_start(writer, e, state)?;
    Ok(())
}

fn handle_text_event<W: Write>(
    text: &BytesText,
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    if state.inside_ctrl {
        state.ctrl_buffer.push(Event::Text(text.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_naglowek {
        state.naglowek_buffer.push(Event::Text(text.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_podmiot {
        state.podmiot_buffer.push(Event::Text(text.clone().into_owned()));
        return Ok(());
    }

    if state.inside_sales {
        state.sales_buffer.push(Event::Text(text.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_purchase {
        state.purchase_buffer.push(Event::Text(text.clone().into_owned()));
        return Ok(());
    }
    
    writer.write_event(Event::Text(text.clone().into_owned()))?;
    Ok(())
}

fn handle_end_event<W: Write>(
    e: &BytesEnd,
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    let full_name = e.name().into_inner();
    let (_, local_name) = split_name(full_name);
    
    if state.inside_ctrl {
        state.ctrl_buffer.push(Event::End(e.clone().into_owned()));
        if local_name == b"SprzedazCtrl" || local_name == b"ZakupCtrl" {
            state.inside_ctrl = false;
            process_ctrl_buffer(state, writer)?;
        }
        return Ok(());
    }
    
    if state.inside_naglowek {
        state.naglowek_buffer.push(Event::End(e.clone().into_owned()));
        if local_name == b"Naglowek" {
            state.inside_naglowek = false;
            process_naglowek_buffer(state, writer)?;
        }
        return Ok(());
    }
    
    if state.inside_podmiot {
        state.podmiot_buffer.push(Event::End(e.clone().into_owned()));
        if local_name == b"Podmiot1" {
            state.inside_podmiot = false;
            process_podmiot_buffer(state, writer)?;
        }
        return Ok(());
    }
    
    if local_name == b"SprzedazWiersz" && state.inside_sales {
        state.sales_buffer.push(Event::End(e.clone().into_owned()));
        let buffer = state.sales_buffer.clone();
        process_row_buffer(state, &buffer, writer, b"DataWystawienia", b"DataSprzedazy", b"TypDokumentu")?;
        state.inside_sales = false;
        return Ok(());
    }

    if local_name == b"ZakupWiersz" && state.inside_purchase {
        state.purchase_buffer.push(Event::End(e.clone().into_owned()));
        let buffer = state.purchase_buffer.clone();
        process_row_buffer(state, &buffer, writer, b"DataZakupu", b"DataWplywu", b"DokumentZakupu")?;
        state.inside_purchase = false;
        return Ok(());
    }

    if state.inside_sales {
        state.sales_buffer.push(Event::End(e.clone().into_owned()));
        return Ok(());
    }
    
    if state.inside_purchase {
        state.purchase_buffer.push(Event::End(e.clone().into_owned()));
        return Ok(());
    }
    
    if local_name == b"JPK" && state.version == FormVersion::V1 && state.has_started_ewidencja {
        let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
        let tag = build_tag(active_prefix, "Ewidencja");
        writer.write_event(Event::End(BytesEnd::new(&tag)))?;
    }

    write_end(writer, e, state)?;
    Ok(())
}

fn handle_empty_event<W: Write>(
    e: &BytesStart,
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    if state.inside_naglowek {
        state.naglowek_buffer.push(Event::Empty(e.clone().into_owned()));
        return Ok(());
    }
    if state.inside_podmiot {
        state.podmiot_buffer.push(Event::Empty(e.clone().into_owned()));
        return Ok(());
    }
    if state.inside_sales {
        state.sales_buffer.push(Event::Empty(e.clone().into_owned()));
        return Ok(());
    }
    if state.inside_purchase {
        state.purchase_buffer.push(Event::Empty(e.clone().into_owned()));
        return Ok(());
    }
    
    write_empty(writer, e, state)?;
    Ok(())
}

fn handle_other_event<W: Write>(
    other: Event,
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    if state.inside_naglowek {
        state.naglowek_buffer.push(other.into_owned());
        return Ok(());
    }
    if state.inside_podmiot {
        state.podmiot_buffer.push(other.into_owned());
        return Ok(());
    }
    if state.inside_sales {
        state.sales_buffer.push(other.into_owned());
        return Ok(());
    }
    if state.inside_purchase {
        state.purchase_buffer.push(other.into_owned());
        return Ok(());
    }
    writer.write_event(other)?;
    Ok(())
}

fn process_naglowek_buffer<W: Write>(
    state: &mut ParserState,
    writer: &mut Writer<W>,
) -> Result<()> {
    let mut explicit_variant = FormVariant::Unknown;
    let children = group_children(&state.naglowek_buffer);

    let mut data_od = String::new();
    let mut data_do = String::new();
    let mut input_kod_urzedu = String::new();
    let mut input_rok = String::new();
    let mut input_miesiac = String::new();
    
    for (name, block) in &children {
        if name == "KodFormularza" || name == "KodFormularzaDekl" {
            if let Some(Event::Start(e)) = block.first() {
                for attr in e.attributes().flatten() {
                    if let Ok(v) = std::str::from_utf8(attr.value.as_ref()) {
                        if v.contains("V7K") { explicit_variant = FormVariant::K; }
                        else if v.contains("V7M") { explicit_variant = FormVariant::M; }
                    }
                }
            }
        }
        if name == "DataOd" {
            for ev in block {
                if let Event::Text(txt) = ev { data_od = String::from_utf8_lossy(txt).trim().to_string(); }
            }
        }
        if name == "DataDo" {
            for ev in block {
                if let Event::Text(txt) = ev { data_do = String::from_utf8_lossy(txt).trim().to_string(); }
            }
        }
        if name == "KodUrzedu" {
            for ev in block {
                if let Event::Text(txt) = ev { input_kod_urzedu = String::from_utf8_lossy(txt).trim().to_string(); }
            }
        }
        if name == "Rok" {
            for ev in block {
                if let Event::Text(txt) = ev { input_rok = String::from_utf8_lossy(txt).trim().to_string(); }
            }
        }
        if name == "Miesiac" {
            for ev in block {
                if let Event::Text(txt) = ev { input_miesiac = String::from_utf8_lossy(txt).trim().to_string(); }
            }
        }
    }

    if state.explicit_variant != FormVariant::Unknown {
        state.variant = state.explicit_variant;
    } else if explicit_variant != FormVariant::Unknown {
        state.variant = explicit_variant;
    } else if data_od.len() >= 7 && data_do.len() >= 7 {
        if data_od[5..7] == data_do[5..7] { state.variant = FormVariant::M; }
        else { state.variant = FormVariant::K; }
    }
    if state.variant == FormVariant::Unknown { state.variant = FormVariant::M; }

    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    let tag = build_tag(active_prefix, "Naglowek");
    writer.write_event(Event::Start(BytesStart::new(&tag)))?;

    // V3 sequence: KodFormularza, WariantFormularza, DataWytworzeniaJPK, NazwaSystemu, CelZlozenia, KodUrzedu, Rok, Miesiac
    let ordered_names = ["KodFormularza", "WariantFormularza", "DataWytworzeniaJPK", "NazwaSystemu", "CelZlozenia", "KodUrzedu", "Rok", "Miesiac"];
    let mut written_names = std::collections::HashSet::new();

    for expected in ordered_names {
        if let Some((_, block)) = children.iter().find(|(n, _)| n == expected) {
            write_block_mapped(writer, block, state, expected)?;
            written_names.insert(expected);
        } else if expected == "KodFormularza" {
            let kf_tag = build_tag(active_prefix, "KodFormularza");
            let mut e_kf = BytesStart::new(&kf_tag);
            if state.variant == FormVariant::K {
                e_kf.push_attribute(("kodSystemowy", "JPK_V7K (3)"));
            } else {
                e_kf.push_attribute(("kodSystemowy", "JPK_V7M (3)"));
            }
            e_kf.push_attribute(("wersjaSchemy", "1-0E"));
            writer.write_event(Event::Start(e_kf))?;
            writer.write_event(Event::Text(BytesText::new("JPK_VAT")))?;
            writer.write_event(Event::End(BytesEnd::new(&kf_tag)))?;
            written_names.insert(expected);
        } else if expected == "WariantFormularza" {
            let wf_tag = build_tag(active_prefix, "WariantFormularza");
            writer.write_event(Event::Start(BytesStart::new(&wf_tag)))?;
            writer.write_event(Event::Text(BytesText::new("3")))?;
            writer.write_event(Event::End(BytesEnd::new(&wf_tag)))?;
            written_names.insert(expected);
        } else if expected == "KodUrzedu" {
            // Priority: CLI arg > Input file
            let ku_val = state.kod_urzedu.as_deref().unwrap_or(&input_kod_urzedu);
            if !ku_val.is_empty() {
                let ku_tag = build_tag(active_prefix, "KodUrzedu");
                writer.write_event(Event::Start(BytesStart::new(&ku_tag)))?;
                writer.write_event(Event::Text(BytesText::new(ku_val)))?;
                writer.write_event(Event::End(BytesEnd::new(&ku_tag)))?;
                written_names.insert(expected);
            } else if state.version == FormVersion::V1 {
                bail!("Error: KodUrzedu is mandatory for conversion to JPK_V7(3). Please provide it using -u or --urzad flag.");
            }
        } else if expected == "Rok" {
            let r_val = if !data_od.is_empty() { &data_od[0..4] } else { &input_rok };
            if !r_val.is_empty() {
                let r_num: i32 = r_val.parse().unwrap_or(0);
                if r_num > 0 {
                    let rok_tag = build_tag(active_prefix, "Rok");
                    writer.write_event(Event::Start(BytesStart::new(&rok_tag)))?;
                    writer.write_event(Event::Text(BytesText::new(r_val)))?;
                    writer.write_event(Event::End(BytesEnd::new(&rok_tag)))?;
                    written_names.insert(expected);
                }
            }
        } else if expected == "Miesiac" {
            let m_val = if !data_od.is_empty() { 
                data_od[5..7].trim_start_matches('0').to_string() 
            } else { 
                input_miesiac.trim_start_matches('0').to_string() 
            };
            if !m_val.is_empty() {
                let mc_tag = build_tag(active_prefix, "Miesiac");
                writer.write_event(Event::Start(BytesStart::new(&mc_tag)))?;
                writer.write_event(Event::Text(BytesText::new(if m_val.is_empty() { "0" } else { &m_val })))?;
                writer.write_event(Event::End(BytesEnd::new(&mc_tag)))?;
                written_names.insert(expected);
            }
        }
    }

    // Drop any other legacy tags from Naglowek (like DataOd, DataDo) by just not writing them.
    
    writer.write_event(Event::End(BytesEnd::new(&tag)))?;
    Ok(())
}

fn process_podmiot_buffer<W: Write>(state: &mut ParserState, writer: &mut Writer<W>) -> Result<()> {
    let children = group_children(&state.podmiot_buffer);
    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    
    let tag = build_tag(active_prefix, "Podmiot1");
    let mut e_pod = BytesStart::new(&tag);
    e_pod.push_attribute(("rola", "Podatnik"));
    writer.write_event(Event::Start(e_pod))?;
    
    let osoba_fiz_block = children.iter().find(|(n, _)| n == "OsobaFizyczna");
    let osoba_niefiz_block = children.iter().find(|(n, _)| n == "OsobaNiefizyczna");
    
    // Capture NIP if present directly or inside person/company blocks
    if let Some((_, block)) = children.iter().find(|(n, _)| n == "NIP") {
        for ev in block {
            if let Event::Text(txt) = ev { state.taxpayer_nip = Some(String::from_utf8_lossy(txt.as_ref()).trim().to_string()); }
        }
    }

    if let Some((_, block)) = osoba_fiz_block {
        let nested = group_children(block);
        if state.taxpayer_nip.is_none() {
            if let Some((_, b)) = nested.iter().find(|(n, _)| n == "NIP") {
                for ev in b {
                    if let Event::Text(txt) = ev { state.taxpayer_nip = Some(String::from_utf8_lossy(txt.as_ref()).trim().to_string()); }
                }
            }
        }
        let o_tag = build_tag(active_prefix, "OsobaFizyczna");
        writer.write_event(Event::Start(BytesStart::new(&o_tag)))?;
        
        // Sequence: NIP, Imie, Nazwisko, DataUrodzenia, Email, Telefon
        let mut email_found = false;
        for expected in ["NIP", "Imie", "Nazwisko", "DataUrodzenia", "Email", "Telefon"] {
            if let Some((_, b)) = nested.iter().find(|(n, _)| n == expected) {
                write_block_mapped(writer, b, state, expected)?;
                if expected == "Email" { email_found = true; }
            } else if expected == "Email" && !email_found {
                write_default_email(writer, active_prefix)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new(&o_tag)))?;
    } else if let Some((_, block)) = osoba_niefiz_block {
        let nested = group_children(block);
        if state.taxpayer_nip.is_none() {
            if let Some((_, b)) = nested.iter().find(|(n, _)| n == "NIP") {
                for ev in b {
                    if let Event::Text(txt) = ev { state.taxpayer_nip = Some(String::from_utf8_lossy(txt.as_ref()).trim().to_string()); }
                }
            }
        }
        let o_tag = build_tag(active_prefix, "OsobaNiefizyczna");
        writer.write_event(Event::Start(BytesStart::new(&o_tag)))?;
        
        // Sequence: NIP, PelnaNazwa, Email, Telefon
        let mut email_found = false;
        for expected in ["NIP", "PelnaNazwa", "Email", "Telefon"] {
            if let Some((_, b)) = nested.iter().find(|(n, _)| n == expected) {
                write_block_mapped(writer, b, state, expected)?;
                if expected == "Email" { email_found = true; }
            } else if expected == "Email" && !email_found {
                write_default_email(writer, active_prefix)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new(&o_tag)))?;
    } else {
        // Flat structure (V1)
        let o_tag = build_tag(active_prefix, "OsobaNiefizyczna");
        writer.write_event(Event::Start(BytesStart::new(&o_tag)))?;
        
        let mut email_found = false;
        // Sequence: NIP, PelnaNazwa, Email, Telefon
        for expected in ["NIP", "PelnaNazwa", "Email", "Telefon"] {
            if let Some((_, block)) = children.iter().find(|(n, _)| {
                n == expected || (expected == "PelnaNazwa" && *n == "NazwaPelna")
            }) {
                write_block_mapped(writer, block, state, expected)?;
                if expected == "Email" { email_found = true; }
            } else if expected == "Email" && !email_found {
                write_default_email(writer, active_prefix)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new(&o_tag)))?;
    }
    
    writer.write_event(Event::End(BytesEnd::new(&tag)))?;
    Ok(())
}

fn write_default_email<W: Write>(writer: &mut Writer<W>, prefix: &[u8]) -> Result<()> {
    let e_tag = build_tag(prefix, "Email");
    writer.write_event(Event::Start(BytesStart::new(&e_tag)))?;
    writer.write_event(Event::Text(BytesText::new("brak@adresu.email")))?;
    writer.write_event(Event::End(BytesEnd::new(&e_tag)))?;
    Ok(())
}

fn write_block_mapped<W: Write>(writer: &mut Writer<W>, block: &[Event<'static>], state: &ParserState, name: &str) -> Result<()> {
    for ev in block {
        match ev {
            Event::Start(e) => {
                let mut e_new = e.clone().into_owned();
                e_new.clear_attributes();
                let mut has_kod_systemowy = false;
                let mut has_wersja_schemy = false;
                let mut has_poz = false;
                for attr in e.attributes().flatten() {
                    let k = attr.key.into_inner();
                    if name == "KodFormularza" && k == b"kodSystemowy" {
                        if state.variant == FormVariant::K {
                            e_new.push_attribute(("kodSystemowy", "JPK_V7K (3)"));
                        } else {
                            e_new.push_attribute(("kodSystemowy", "JPK_V7M (3)"));
                        }
                        has_kod_systemowy = true;
                    } else if name == "KodFormularza" && k == b"wersjaSchemy" {
                        e_new.push_attribute(("wersjaSchemy", "1-0E"));
                        has_wersja_schemy = true;
                    } else if name == "CelZlozenia" && k == b"poz" {
                        e_new.push_attribute(("poz", "P_7"));
                        has_poz = true;
                    } else {
                        e_new.push_attribute(attr);
                    }
                }
                if name == "KodFormularza" {
                    if !has_kod_systemowy {
                        if state.variant == FormVariant::K {
                            e_new.push_attribute(("kodSystemowy", "JPK_V7K (3)"));
                        } else {
                            e_new.push_attribute(("kodSystemowy", "JPK_V7M (3)"));
                        }
                    }
                    if !has_wersja_schemy {
                        e_new.push_attribute(("wersjaSchemy", "1-0E"));
                    }
                }
                if name == "CelZlozenia" && !has_poz {
                    e_new.push_attribute(("poz", "P_7"));
                }
                if name == "KodFormularza" {
                    // Force JPK_VAT inside block? No, done via text
                }
                write_start(writer, &e_new, state)?;
            }
            Event::Text(txt) => {
                if name == "KodFormularza" || name == "KodFormularzaDekl" {
                    writer.write_event(Event::Text(BytesText::new("JPK_VAT")))?;
                } else if name == "WariantFormularza" {
                    writer.write_event(Event::Text(BytesText::new("3")))?;
                } else {
                    writer.write_event(Event::Text(txt.clone().into_owned()))?;
                }
            }
            Event::Empty(e) => { write_empty(writer, e, state)?; }
            Event::End(e) => { write_end(writer, e, state)?; }
            other => { writer.write_event(other.clone())?; }
        }
    }
    Ok(())
}

fn process_row_buffer<W: Write>(
    state: &mut ParserState,
    buffer: &[Event<'static>],
    writer: &mut Writer<W>,
    date1_tag: &[u8],
    date2_tag: &[u8],
    doc_type_tag: &[u8],
) -> Result<()> {
    let mut insert_index = 0;
    
    for (i, ev) in buffer.iter().enumerate() {
        if let Event::End(e) = ev {
            let (_, name) = split_name(e.name().into_inner());
            if name == date1_tag || name == date2_tag {
                insert_index = i + 1;
            }
        }
    }
    
    let mut doc_type_val = String::new();
    for (i, ev) in buffer.iter().enumerate() {
        if let Event::Start(e) = ev {
            let (_, name) = split_name(e.name().into_inner());
            if name == doc_type_tag {
                if let Some(Event::Text(txt)) = buffer.get(i + 1) {
                    if let Ok(val) = std::str::from_utf8(txt.as_ref()) {
                        doc_type_val = val.trim().to_string();
                    }
                }
            }
        }
    }
    
    let is_fp = doc_type_val == "FP";

    let mut row_base = 0.0;
    let mut row_vat = 0.0;
    
    for (i, ev) in buffer.iter().enumerate() {
        if let Event::Start(e) = ev {
            let (_, name) = split_name(e.name().into_inner());
            let name_str = std::str::from_utf8(name).unwrap_or("");
            
            if name_str.starts_with("K_") {
                if let Some(Event::Text(txt)) = buffer.get(i + 1) {
                    if let Ok(val_str) = std::str::from_utf8(txt.as_ref()) {
                        if let Ok(val) = val_str.trim().parse::<f64>() {
                            match name_str {
                                "K_10" | "K_11" | "K_13" | "K_15" | "K_17" | "K_19" | "K_21" | "K_23" | "K_25" | "K_27" | "K_29" | "K_31" => {
                                    if !is_fp { 
                                        row_base += val;
                                        if date1_tag == b"DataWystawienia" { // Sales
                                            match name_str {
                                                "K_15" => state.breakdown.base_5 += val,
                                                "K_17" => state.breakdown.base_8 += val,
                                                "K_19" => state.breakdown.base_23 += val,
                                                _ => state.breakdown.base_other += val,
                                            }
                                        }
                                    }
                                }
                                "K_16" | "K_18" | "K_20" | "K_22" | "K_24" | "K_26" | "K_28" | "K_30" | "K_32" | "K_33" | "K_34" | "K_35" | "K_36" | "K_360" => {
                                    if !is_fp {
                                        row_vat += val;
                                        if date1_tag == b"DataWystawienia" { // Sales
                                            match name_str {
                                                "K_16" => state.breakdown.vat_5 += val,
                                                "K_18" => state.breakdown.vat_8 += val,
                                                "K_20" => state.breakdown.vat_23 += val,
                                                _ => state.breakdown.vat_other += val,
                                            }
                                        }
                                    }
                                }
                                "K_40" | "K_42" => { // Purchase Base
                                    row_base += val;
                                }
                                "K_41" | "K_43" | "K_44" | "K_45" | "K_46" | "K_47" => { // Purchase VAT
                                    row_vat += val;
                                }
                                _ => {}
                            }
                            
                            // Legacy accumulators for control sections
                            match name_str {
                                "K_16" => if !is_fp { state.sum_k16 += val },
                                "K_18" => if !is_fp { state.sum_k18 += val },
                                "K_20" => if !is_fp { state.sum_k20 += val },
                                "K_24" => if !is_fp { state.sum_k24 += val },
                                "K_26" => if !is_fp { state.sum_k26 += val },
                                "K_28" => if !is_fp { state.sum_k28 += val },
                                "K_30" => if !is_fp { state.sum_k30 += val },
                                "K_32" => if !is_fp { state.sum_k32 += val },
                                "K_33" => if !is_fp { state.sum_k33 += val },
                                "K_34" => if !is_fp { state.sum_k34 += val },
                                "K_35" => if !is_fp { state.sum_k35 += val },
                                "K_36" => if !is_fp { state.sum_k36 += val },
                                "K_360" => if !is_fp { state.sum_k360 += val },
                                "K_41" => state.sum_k41 += val,
                                "K_43" => state.sum_k43 += val,
                                "K_44" => state.sum_k44 += val,
                                "K_45" => state.sum_k45 += val,
                                "K_46" => state.sum_k46 += val,
                                "K_47" => state.sum_k47 += val,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    if !is_fp {
        if date1_tag == b"DataWystawienia" { // Sales
            state.count_sales += 1;
            state.total_sales_base += row_base;
            state.total_sales_vat += row_vat;
            let count = state.count_sales;
            if state.max_sales_vat.map_or(true, |(v, _)| row_vat > v) {
                state.max_sales_vat = Some((row_vat, count));
            }
            if state.min_sales_vat.map_or(true, |(v, _)| row_vat < v) {
                state.min_sales_vat = Some((row_vat, count));
            }
        } else { // Purchase
            state.count_purchase += 1;
            state.total_purchase_base += row_base;
            state.total_purchase_vat += row_vat;
            let count = state.count_purchase;
            if state.max_purchase_vat.map_or(true, |(v, _)| row_vat > v) {
                state.max_purchase_vat = Some((row_vat, count));
            }
            if state.min_purchase_vat.map_or(true, |(v, _)| row_vat < v) {
                state.min_purchase_vat = Some((row_vat, count));
            }
        }
    }

    let use_di = doc_type_val == "WEW" || doc_type_val == "RO";

    let insert_tag_name = if use_di { "DI" } else { "BFK" };
    let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
    
    let mut skip_depth = 0;
    let mut choice_inserted = false;
    
    for (i, ev) in buffer.iter().enumerate() {
        if i == insert_index && insert_index > 0 {
            let tag = build_tag(active_prefix, insert_tag_name);
            writer.write_event(Event::Start(BytesStart::new(&tag)))?;
            writer.write_event(Event::Text(BytesText::new("1")))?;
            writer.write_event(Event::End(BytesEnd::new(&tag)))?;
            choice_inserted = true;
        }
        match ev {
            Event::Start(e) => {
                let (_, name) = split_name(e.name().into_inner());
                if name == b"AdresKontrahenta" || name == b"AdresDostawcy" {
                    skip_depth += 1;
                    continue;
                }
                if name == b"DI" || name == b"BFK" || name == b"OFF" || name == b"NrKSeF" {
                    // Already exists? Skip our insertion later
                    choice_inserted = true;
                }
                if skip_depth > 0 { skip_depth += 1; continue; }
                write_start(writer, e, state)?;
            }
            Event::End(e) => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
                write_end(writer, e, state)?;
            }
            Event::Empty(e) => {
                let (_, name) = split_name(e.name().into_inner());
                if name == b"AdresKontrahenta" || name == b"AdresDostawcy" { continue; }
                if skip_depth > 0 { continue; }
                write_empty(writer, e, state)?;
            }
            Event::Text(_) | Event::CData(_) => {
                if skip_depth > 0 { continue; }
                writer.write_event(ev.clone())?;
            }
            other => {
                if skip_depth > 0 { continue; }
                writer.write_event(other.clone())?;
            }
        }
    }
    
    // Safety if insert_index was not found (shouldn't happen with valid JPK)
    if !choice_inserted {
        let tag = build_tag(active_prefix, insert_tag_name);
        writer.write_event(Event::Start(BytesStart::new(&tag)))?;
        writer.write_event(Event::Text(BytesText::new("1")))?;
        writer.write_event(Event::End(BytesEnd::new(&tag)))?;
    }
    
    Ok(())
}

fn process_ctrl_buffer<W: Write>(state: &ParserState, writer: &mut Writer<W>) -> Result<()> {
    let children = group_children(&state.ctrl_buffer);
    if let Some(Event::Start(e)) = state.ctrl_buffer.first() {
        let (_, local) = split_name(e.name().into_inner());
        let active_prefix = state.target_prefix.as_deref().unwrap_or(&state.root_prefix);
        let tag = build_tag(active_prefix, std::str::from_utf8(local).unwrap_or(""));
        writer.write_event(Event::Start(BytesStart::new(&tag)))?;
        
        if local == b"SprzedazCtrl" {
            // Count
            let cnt_tag = build_tag(active_prefix, "LiczbaWierszySprzedazy");
            writer.write_event(Event::Start(BytesStart::new(&cnt_tag)))?;
            writer.write_event(Event::Text(BytesText::new(&state.count_sales.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new(&cnt_tag)))?;
            
            // Total
            let calculated_total = state.sum_k16 + state.sum_k18 + state.sum_k20 + state.sum_k24 + 
                                  state.sum_k26 + state.sum_k28 + state.sum_k30 + state.sum_k32 + 
                                  state.sum_k33 + state.sum_k34 - state.sum_k35 - state.sum_k36 - state.sum_k360;
            
            let mut old_val = String::new();
            if let Some((_, block)) = children.iter().find(|(n, _)| n == "PodatekNalezny") {
                for ev in block {
                    if let Event::Text(txt) = ev { old_val = String::from_utf8_lossy(txt).trim().to_string(); }
                }
            }
            
            let total_str = format!("{:.2}", calculated_total);
            let mut final_val = total_str.clone();
            
            if !old_val.is_empty() && old_val != total_str && old_val != format!("{:.1}", calculated_total) {
                let msg = format!("Warning: Control total discrepancy in SprzedazCtrl/PodatekNalezny is {} (calculated {})", old_val, total_str);
                eprintln!("\x1b[33m{}\x1b[0m", msg);
                writer.write_event(Event::Comment(BytesText::new(&format!(" {} ", msg))))?;
                final_val = old_val;
            }
            
            let tot_tag = build_tag(active_prefix, "PodatekNalezny");
            writer.write_event(Event::Start(BytesStart::new(&tot_tag)))?;
            writer.write_event(Event::Text(BytesText::new(&final_val)))?;
            writer.write_event(Event::End(BytesEnd::new(&tot_tag)))?;
        } else if local == b"ZakupCtrl" {
            // Count
            let cnt_tag = build_tag(active_prefix, "LiczbaWierszyZakupow");
            writer.write_event(Event::Start(BytesStart::new(&cnt_tag)))?;
            writer.write_event(Event::Text(BytesText::new(&state.count_purchase.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new(&cnt_tag)))?;
            
            // Total
            let calculated_total = state.sum_k41 + state.sum_k43 + state.sum_k44 + state.sum_k45 + state.sum_k46 + state.sum_k47;
            
            let mut old_val = String::new();
            if let Some((_, block)) = children.iter().find(|(n, _)| n == "PodatekNaliczony") {
                for ev in block {
                    if let Event::Text(txt) = ev { old_val = String::from_utf8_lossy(txt).trim().to_string(); }
                }
            }
            
            let total_str = format!("{:.2}", calculated_total);
            let mut final_val = total_str.clone();
            
            if !old_val.is_empty() && old_val != total_str && old_val != format!("{:.1}", calculated_total) {
                let msg = format!("Warning: Control total discrepancy in ZakupCtrl/PodatekNaliczony is {} (calculated {})", old_val, total_str);
                eprintln!("\x1b[33m{}\x1b[0m", msg);
                writer.write_event(Event::Comment(BytesText::new(&format!(" {} ", msg))))?;
                final_val = old_val;
            }
            
            let tot_tag = build_tag(active_prefix, "PodatekNaliczony");
            writer.write_event(Event::Start(BytesStart::new(&tot_tag)))?;
            writer.write_event(Event::Text(BytesText::new(&final_val)))?;
            writer.write_event(Event::End(BytesEnd::new(&tot_tag)))?;
        }
        
        writer.write_event(Event::End(BytesEnd::new(&tag)))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../test/jpk23_tests.rs"]
mod tests;
