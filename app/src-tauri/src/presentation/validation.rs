use super::{Control, Error, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};
use zip::{CompressionMethod, ZipArchive};

pub const MAX_SOURCE: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
const MAX_PART: u64 = 128 * 1024 * 1024;
const MAX_XML: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 5000;
pub const MAX_SLIDES: usize = 100;

#[derive(Debug)]
pub struct Format {
    pub count: usize,
    pub emu: (u64, u64),
}

impl Format {
    pub fn raster_dimensions(&self) -> (u32, u32) {
        let scale = (1920.0 / self.emu.0 as f64).min(1080.0 / self.emu.1 as f64);
        (
            (self.emu.0 as f64 * scale).round() as u32,
            (self.emu.1 as f64 * scale).round() as u32,
        )
    }
}

fn invalid_archive(error: impl std::fmt::Display) -> Error {
    Error::invalid(format!(
        "Invalid, encrypted, or unsupported PowerPoint package. {error}"
    ))
}

pub fn validate(path: &Path, control: &Control) -> Result<Format> {
    let file = File::open(path).map_err(Error::access)?;
    validate_archive(file, false, &mut 0, control)
}

fn validate_archive(
    mut source: impl Read + Seek,
    workbook: bool,
    expanded: &mut u64,
    control: &Control,
) -> Result<Format> {
    let entry_count = bounded_directory(&mut source)?;
    let mut archive = ZipArchive::new(source).map_err(invalid_archive)?;
    if archive.len() != entry_count || archive.offset() != 0 {
        return Err(invalid_archive(
            "Unexpected archive layout or too many parts.",
        ));
    }
    archive
        .validate_header_consistency()
        .map_err(invalid_archive)?;
    let mut names = HashSet::new();
    let mut buffer = [0; 64 * 1024];
    let mut format = Format {
        count: 0,
        emu: (0, 0),
    };
    let mut has_content_type = false;
    let mut has_root = false;
    for index in 0..archive.len() {
        control.check()?;
        let mut entry = archive.by_index(index).map_err(invalid_archive)?;
        let name = entry.name().map_err(invalid_archive)?.to_ascii_lowercase();
        if entry.enclosed_name().is_none()
            || entry.is_symlink()
            || !safe_part(&name)
            || !names.insert(name.trim_end_matches('/').to_owned())
        {
            return Err(invalid_archive("Unsafe or duplicate part name."));
        }
        let chart_data =
            !workbook && name.starts_with("ppt/embeddings/") && name.ends_with(".xlsx");
        if name.starts_with("ppt/activex/")
            || name.starts_with("xl/activex/")
            || name.starts_with("xl/embeddings/")
            || name.starts_with("xl/externallinks/")
            || name.starts_with("xl/querytables/")
            || name == "xl/connections.xml"
            || name.starts_with("customui/")
            || name.contains("vbaproject")
            || name.ends_with(".vbs")
            || (!entry.is_dir() && name.starts_with("ppt/embeddings/") && !chart_data)
        {
            return Err(Error::invalid(
                "PowerPoint files with macros, external chart data, or active embedded objects are unsupported.",
            ));
        }
        if entry.is_dir() {
            continue;
        }
        let xml = name.ends_with(".xml") || name.ends_with(".rels");
        let limit = if xml || chart_data { MAX_XML } else { MAX_PART };
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) || entry.size() > limit
            || entry.size() > MAX_EXPANDED - *expanded
        {
            return Err(invalid_archive(
                "Unsupported compression or expanded size limit exceeded.",
            ));
        }
        let mut size = 0;
        let mut bytes = Vec::new();
        loop {
            control.check()?;
            let count = entry.read(&mut buffer).map_err(invalid_archive)?;
            if count == 0 {
                break;
            }
            size += count as u64;
            *expanded += count as u64;
            if *expanded > MAX_EXPANDED || size > limit || size > entry.size() {
                return Err(invalid_archive("Expanded size limit exceeded."));
            }
            if xml || chart_data {
                bytes.extend_from_slice(&buffer[..count]);
            }
        }
        if size != entry.size() {
            return Err(invalid_archive("Incorrect part size."));
        }
        if xml {
            inspect_xml(
                &name,
                &bytes,
                &mut format,
                &mut has_content_type,
                &mut has_root,
                control,
            )?;
        } else if chart_data {
            validate_archive(Cursor::new(bytes), true, expanded, control)?;
        }
    }
    if workbook {
        if !names.contains("xl/workbook.xml") || !names.contains("[content_types].xml") {
            return Err(invalid_archive("An embedded chart workbook is invalid."));
        }
        return Ok(format);
    }
    let (width, height) = format.emu;
    if !has_content_type
        || !has_root
        || !names.contains("ppt/presentation.xml")
        || !(1..=MAX_SLIDES).contains(&format.count)
        || !(91_440..=51_206_400).contains(&width)
        || !(91_440..=51_206_400).contains(&height)
    {
        return Err(Error::invalid("Choose an unencrypted .pptx presentation with 1–100 slides and valid slide dimensions."));
    }
    Ok(format)
}

fn bounded_directory(source: &mut (impl Read + Seek)) -> Result<usize> {
    // Check the entry count before the ZIP reader allocates a central-directory index.
    let length = source.seek(SeekFrom::End(0)).map_err(invalid_archive)?;
    let window = length.min(22 + u64::from(u16::MAX));
    source
        .seek(SeekFrom::Start(length - window))
        .map_err(invalid_archive)?;
    let mut tail = vec![0; window as usize];
    source.read_exact(&mut tail).map_err(invalid_archive)?;
    let last = tail.len().saturating_sub(22);
    for start in (0..=last).rev() {
        let footer = &tail[start..];
        if footer.len() < 22 || !footer.starts_with(b"PK\x05\x06") {
            continue;
        }
        let field = |offset| u16::from_le_bytes([footer[offset], footer[offset + 1]]);
        if footer.len() != 22 + usize::from(field(20)) {
            continue;
        }
        let count = usize::from(field(10));
        if field(4) != 0
            || field(6) != 0
            || field(8) != field(10)
            || !(1..=MAX_ENTRIES).contains(&count)
        {
            return Err(invalid_archive(
                "Too many archive parts or unsupported ZIP directory.",
            ));
        }
        let size = u32::from_le_bytes(footer[12..16].try_into().unwrap());
        let offset = u32::from_le_bytes(footer[16..20].try_into().unwrap());
        if u64::from(size) + u64::from(offset) != length - window + start as u64 {
            return Err(invalid_archive("Invalid archive directory bounds."));
        }
        return Ok(count);
    }
    Err(invalid_archive("Missing archive directory."))
}

fn safe_part(name: &str) -> bool {
    !name.contains(['\\', ':'])
        && !name.chars().any(char::is_control)
        && name
            .trim_end_matches('/')
            .split('/')
            .all(|part| !part.is_empty() && !part.ends_with(['.', ' ']))
}

fn attributes<'a>(event: &BytesStart<'a>) -> Result<HashMap<String, String>> {
    event
        .attributes()
        .map(|attribute| {
            let attribute = attribute.map_err(invalid_archive)?;
            let value = attribute
                .normalized_value(quick_xml::XmlVersion::Explicit1_0)
                .map_err(invalid_archive)?;
            Ok((attribute.key.as_ref().to_owned(), value.into_owned()))
        })
        .collect()
}

fn inspect_xml(
    part: &str,
    bytes: &[u8],
    format: &mut Format,
    has_content_type: &mut bool,
    has_root: &mut bool,
    control: &Control,
) -> Result<()> {
    // OOXML producers normally use UTF-8. Reject other encodings before inspecting links.
    let xml = std::str::from_utf8(bytes).map_err(invalid_archive)?;
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut root_seen = false;
    loop {
        control.check()?;
        let event = reader.read_event().map_err(invalid_archive)?;
        let (element, start) = match event {
            Event::Start(element) => (element, true),
            Event::Empty(element) => (element, false),
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err(invalid_archive("Unexpected closing tag."));
                }
                continue;
            }
            Event::DocType(_) => {
                return Err(invalid_archive(
                    "Document type declarations are unsupported.",
                ))
            }
            Event::Decl(decl) => {
                if let Some(encoding) = decl.encoding() {
                    if !encoding
                        .map_err(invalid_archive)?
                        .eq_ignore_ascii_case("utf-8")
                    {
                        return Err(invalid_archive("Unsupported XML encoding."));
                    }
                }
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let local = element.local_name();
        let name = local.as_ref();
        if stack.is_empty() {
            if root_seen {
                return Err(invalid_archive("Multiple XML document elements."));
            }
            root_seen = true;
        }
        if part.ends_with(".rels") && name == "Relationship" {
            let attrs = attributes(&element)?;
            let link_type = attrs
                .get("Type")
                .ok_or_else(|| invalid_archive("Missing relationship type."))?;
            let target = attrs
                .get("Target")
                .ok_or_else(|| invalid_archive("Missing relationship target."))?;
            let mode = attrs
                .get("TargetMode")
                .map(String::as_str)
                .unwrap_or("Internal");
            let link_kind = link_type
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            let chart_workbook = part.starts_with("ppt/charts/_rels/")
                && target.to_ascii_lowercase().starts_with("../embeddings/")
                && target.to_ascii_lowercase().ends_with(".xlsx")
                && target.split('/').count() == 3;
            if matches!(
                link_kind.as_str(),
                "vbaproject" | "oleobject" | "activexcontrol" | "control" | "afchunk"
            ) || (link_kind == "package" && !chart_workbook)
            {
                return Err(Error::invalid(
                    "PowerPoint files with active embedded objects are unsupported.",
                ));
            }
            // Ordinary clickable links never run during a static export.
            let hyperlink = mode == "External"
                && [
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
                    "http://purl.oclc.org/ooxml/officeDocument/relationships/hyperlink",
                ]
                .contains(&link_type.as_str())
                && ["https://", "http://", "mailto:"]
                    .iter()
                    .any(|prefix| target.to_ascii_lowercase().starts_with(prefix));
            if !hyperlink && (mode != "Internal" || !internal_target(part, target)) {
                return Err(Error::invalid("This PowerPoint links to external content. Embed the linked media or export a static PDF."));
            }
            if part == "_rels/.rels"
                && link_kind == "officedocument"
                && target.trim_start_matches('/') == "ppt/presentation.xml"
            {
                *has_root = true;
            }
        } else if part == "[content_types].xml" && matches!(name, "Override" | "Default") {
            let attrs = attributes(&element)?;
            let mime = attrs
                .get("ContentType")
                .ok_or_else(|| invalid_archive("Missing content type."))?;
            if mime.to_ascii_lowercase().contains("macroenabled")
                || mime.to_ascii_lowercase().contains("vbaproject")
            {
                return Err(Error::invalid(
                    "Macro-enabled PowerPoint presentations are unsupported.",
                ));
            }
            if attrs.get("PartName").is_some_and(|part| part == "/ppt/presentation.xml")
                && mime == "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
            {
                *has_content_type = true;
            }
        } else if part == "ppt/presentation.xml"
            && stack.first().is_some_and(|root| root == "presentation")
        {
            if name == "sldId" && stack.len() == 2 && stack[1] == "sldIdLst" {
                format.count += 1;
                if format.count > MAX_SLIDES {
                    return Err(Error::invalid("A PowerPoint file must have 1–100 slides."));
                }
            } else if name == "sldSz" && stack.len() == 1 {
                let attrs = attributes(&element)?;
                let dim = |key: &str| {
                    attrs
                        .get(key)
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0)
                };
                format.emu = (dim("cx"), dim("cy"));
            }
        }
        if start {
            if stack.len() >= 256 {
                return Err(invalid_archive("XML is nested too deeply."));
            }
            stack.push(name.to_owned());
        }
    }
    if !root_seen || !stack.is_empty() {
        return Err(invalid_archive("Unclosed XML tag."));
    }
    Ok(())
}

fn internal_target(part: &str, target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    if target.is_empty()
        || target.starts_with("//")
        || target.contains(['\\', ':'])
        || target.chars().any(char::is_control)
        || ["%2e", "%2f", "%5c", "%3a", "%00", "%25"]
            .iter()
            .any(|escape| lower.contains(escape))
    {
        return false;
    }
    let mut depth = if target.starts_with('/') {
        0
    } else {
        part.rsplit_once("_rels/")
            .map_or(0, |(prefix, _)| prefix.matches('/').count())
    };
    for component in target.split('#').next().unwrap_or("").split('/') {
        match component {
            "" | "." => {}
            ".." if depth == 0 => return false,
            ".." => depth -= 1,
            _ => depth += 1,
        }
    }
    true
}
