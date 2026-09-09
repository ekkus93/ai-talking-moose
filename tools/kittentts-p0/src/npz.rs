//! Minimal P0-only NumPy NPZ/NPY reader for Kitten voice embeddings.
//!
//! Derived from the Apache-2.0 `kittentts-rs` 0.4.1 implementation and kept
//! deliberately narrow: float32, C-contiguous arrays, NPY v1/v2, ZIP/deflate.

use anyhow::{bail, Context, Result};
use std::{collections::HashMap, io::Read, path::Path};
use zip::ZipArchive;

pub struct NpyArray {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl NpyArray {
    pub fn nrows(&self) -> usize {
        self.shape.first().copied().unwrap_or(0)
    }

    pub fn ncols(&self) -> usize {
        self.shape.get(1).copied().unwrap_or(1)
    }
}

fn extract_header_field<'a>(header: &'a str, field: &str) -> Option<&'a str> {
    let single = format!("'{field}':");
    let double = format!("\"{field}\":");
    let start = header
        .find(&single)
        .map(|offset| offset + single.len())
        .or_else(|| header.find(&double).map(|offset| offset + double.len()))?;
    let rest = header[start..].trim_start();

    if rest.starts_with('(') {
        let end = rest.find(')')?;
        Some(&rest[..=end])
    } else if rest.starts_with('\'') || rest.starts_with('"') {
        let quote = rest.chars().next()?;
        let inner = &rest[1..];
        let end = inner.find(quote)?;
        Some(&inner[..end])
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

fn parse_shape(value: &str) -> Result<Vec<usize>> {
    let inner = value.trim().trim_start_matches('(').trim_end_matches(')');
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<usize>()
                .with_context(|| format!("invalid NPY shape dimension {part:?}"))
        })
        .collect()
}

fn parse_npy(bytes: &[u8]) -> Result<NpyArray> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        bail!("invalid NPY magic");
    }

    let major = bytes[6];
    let minor = bytes[7];
    let (header_len, header_start) = match (major, minor) {
        (1, _) => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
        (2, _) => {
            if bytes.len() < 12 {
                bail!("truncated NPY v2 header");
            }
            (
                u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
                12,
            )
        }
        _ => bail!("unsupported NPY version {major}.{minor}"),
    };

    let header_end = header_start + header_len;
    if header_end > bytes.len() {
        bail!("truncated NPY header");
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end])
        .context("NPY header is not UTF-8")?;

    let dtype = extract_header_field(header, "descr")
        .context("NPY header has no descr")?
        .trim()
        .trim_matches('\'')
        .trim_matches('"');
    if !matches!(dtype, "<f4" | "=f4" | "|f4" | ">f4") {
        bail!("unsupported NPY dtype {dtype:?}");
    }
    if extract_header_field(header, "fortran_order")
        .unwrap_or("False")
        .trim()
        .eq_ignore_ascii_case("true")
    {
        bail!("Fortran-order NPY arrays are unsupported");
    }

    let shape = parse_shape(extract_header_field(header, "shape").context("NPY header has no shape")?)?;
    let count: usize = shape.iter().product();
    let data_bytes = &bytes[header_end..];
    if data_bytes.len() < count * 4 {
        bail!("truncated NPY data");
    }

    let big_endian = dtype.starts_with('>');
    let data = data_bytes[..count * 4]
        .chunks_exact(4)
        .map(|chunk| {
            let raw = [chunk[0], chunk[1], chunk[2], chunk[3]];
            if big_endian {
                f32::from_be_bytes(raw)
            } else {
                f32::from_le_bytes(raw)
            }
        })
        .collect();

    Ok(NpyArray { shape, data })
}

pub fn load_npz(path: &Path) -> Result<HashMap<String, NpyArray>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("cannot open NPZ file {}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("cannot open NPZ archive {}", path.display()))?;
    let mut arrays = HashMap::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("cannot read NPZ entry")?;
        let name = entry.name().trim_end_matches(".npy").to_string();
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).context("cannot read NPY payload")?;
        arrays.insert(name, parse_npy(&bytes).with_context(|| format!("invalid NPY entry at index {index}"))?);
    }

    Ok(arrays)
}
