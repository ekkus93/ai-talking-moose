use super::LocalTtsRuntimeError;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

const MAX_NPZ_ENTRIES: usize = 64;
const MAX_NPY_BYTES: u64 = 16 * 1024 * 1024;

pub(super) struct NpyArray {
    pub(super) shape: Vec<usize>,
    pub(super) data: Vec<f32>,
}

impl NpyArray {
    pub(super) fn nrows(&self) -> usize {
        self.shape.first().copied().unwrap_or(0)
    }

    pub(super) fn ncols(&self) -> usize {
        self.shape.get(1).copied().unwrap_or(1)
    }
}

fn fail<T>() -> Result<T, LocalTtsRuntimeError> {
    Err(LocalTtsRuntimeError::model_load())
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

fn parse_shape(value: &str) -> Result<Vec<usize>, LocalTtsRuntimeError> {
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
                .map_err(|_| LocalTtsRuntimeError::model_load())
        })
        .collect()
}

fn parse_npy(bytes: &[u8]) -> Result<NpyArray, LocalTtsRuntimeError> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return fail();
    }

    let major = bytes[6];
    let minor = bytes[7];
    let (header_len, header_start): (usize, usize) = match (major, minor) {
        (1, _) => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
        (2, _) if bytes.len() >= 12 => (
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
            12,
        ),
        _ => return fail(),
    };

    let header_end = header_start
        .checked_add(header_len)
        .ok_or_else(LocalTtsRuntimeError::model_load)?;
    if header_end > bytes.len() {
        return fail();
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end])
        .map_err(|_| LocalTtsRuntimeError::model_load())?;

    let dtype = extract_header_field(header, "descr")
        .ok_or_else(LocalTtsRuntimeError::model_load)?
        .trim()
        .trim_matches('\'')
        .trim_matches('"');
    if !matches!(dtype, "<f4" | "=f4" | "|f4" | ">f4") {
        return fail();
    }
    if extract_header_field(header, "fortran_order")
        .unwrap_or("False")
        .trim()
        .eq_ignore_ascii_case("true")
    {
        return fail();
    }

    let shape = parse_shape(
        extract_header_field(header, "shape").ok_or_else(LocalTtsRuntimeError::model_load)?,
    )?;
    if shape.len() != 2 || shape.iter().any(|dimension| *dimension == 0) {
        return fail();
    }
    let count = shape.iter().try_fold(1usize, |count, dimension| {
        count
            .checked_mul(*dimension)
            .ok_or_else(LocalTtsRuntimeError::model_load)
    })?;
    let byte_count = count
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(LocalTtsRuntimeError::model_load)?;
    let data_bytes = bytes
        .get(header_end..)
        .ok_or_else(LocalTtsRuntimeError::model_load)?;
    if data_bytes.len() < byte_count {
        return fail();
    }

    let big_endian = dtype.starts_with('>');
    let data = data_bytes[..byte_count]
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

pub(super) fn load_npz(path: &Path) -> Result<HashMap<String, NpyArray>, LocalTtsRuntimeError> {
    let file = std::fs::File::open(path).map_err(|_| LocalTtsRuntimeError::model_load())?;
    let mut archive = ZipArchive::new(file).map_err(|_| LocalTtsRuntimeError::model_load())?;
    if archive.len() == 0 || archive.len() > MAX_NPZ_ENTRIES {
        return fail();
    }

    let mut arrays = HashMap::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        if !entry.is_file() || entry.size() == 0 || entry.size() > MAX_NPY_BYTES {
            return fail();
        }
        let name = entry
            .name()
            .strip_suffix(".npy")
            .ok_or_else(LocalTtsRuntimeError::model_load)?
            .to_string();
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || arrays.contains_key(&name)
        {
            return fail();
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut bytes)
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        arrays.insert(name, parse_npy(&bytes)?);
    }
    Ok(arrays)
}
