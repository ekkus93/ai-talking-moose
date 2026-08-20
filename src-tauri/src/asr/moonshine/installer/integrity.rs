use super::super::manifest::MoonshineModelFile;
use super::transport::DownloadSink;
use super::MoonshineModelInstallError;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ring::digest::{Context as Sha256Context, SHA256};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(super) struct Crc32c {
    state: u32,
}

impl Default for Crc32c {
    fn default() -> Self {
        Self { state: u32::MAX }
    }
}

impl Crc32c {
    pub(super) fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let index = ((self.state ^ u32::from(byte)) & 0xff) as usize;
            self.state = CRC32C_TABLE[index] ^ (self.state >> 8);
        }
    }

    pub(super) fn finalize(self) -> u32 {
        !self.state
    }
}

const fn crc32c_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut index = 0_usize;
    while index < table.len() {
        let mut value = index as u32;
        let mut bit = 0_u8;
        while bit < 8 {
            value = if value & 1 == 1 {
                (value >> 1) ^ 0x82f6_3b78
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

const CRC32C_TABLE: [u32; 256] = crc32c_table();

pub(super) fn crc32c_base64(value: u32) -> String {
    BASE64_STANDARD.encode(value.to_be_bytes())
}

pub(super) fn digest_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub(super) struct VerifyingFileSink<'a> {
    file: File,
    manifest_file: &'a MoonshineModelFile,
    bytes_written: u64,
    sha256: Sha256Context,
    crc32c: Crc32c,
}

impl<'a> VerifyingFileSink<'a> {
    pub(super) fn create(
        path: &Path,
        manifest_file: &'a MoonshineModelFile,
    ) -> Result<Self, MoonshineModelInstallError> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| MoonshineModelInstallError::io("create the staging file"))?;
        Ok(Self {
            file,
            manifest_file,
            bytes_written: 0,
            sha256: Sha256Context::new(&SHA256),
            crc32c: Crc32c::default(),
        })
    }

    pub(super) fn finish(mut self) -> Result<(), MoonshineModelInstallError> {
        self.file
            .flush()
            .and_then(|()| self.file.sync_all())
            .map_err(|_| MoonshineModelInstallError::io("flush the staging file"))?;
        if self.bytes_written != self.manifest_file.bytes {
            return Err(MoonshineModelInstallError::size_mismatch(
                self.manifest_file.name,
            ));
        }

        let sha256 = digest_hex(self.sha256.finish().as_ref());
        if !sha256.eq_ignore_ascii_case(self.manifest_file.sha256) {
            return Err(MoonshineModelInstallError::sha256_mismatch(
                self.manifest_file.name,
            ));
        }
        if crc32c_base64(self.crc32c.finalize()) != self.manifest_file.upstream_crc32c_base64 {
            return Err(MoonshineModelInstallError::crc32c_mismatch(
                self.manifest_file.name,
            ));
        }
        Ok(())
    }
}

impl DownloadSink for VerifyingFileSink<'_> {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), MoonshineModelInstallError> {
        let chunk_bytes = u64::try_from(chunk.len())
            .map_err(|_| MoonshineModelInstallError::size_mismatch(self.manifest_file.name))?;
        let next_size = self.bytes_written.saturating_add(chunk_bytes);
        if next_size > self.manifest_file.bytes {
            return Err(MoonshineModelInstallError::size_mismatch(
                self.manifest_file.name,
            ));
        }
        self.file
            .write_all(chunk)
            .map_err(|_| MoonshineModelInstallError::io("write the staging file"))?;
        self.sha256.update(chunk);
        self.crc32c.update(chunk);
        self.bytes_written = next_size;
        Ok(())
    }
}
