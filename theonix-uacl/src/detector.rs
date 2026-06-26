use anyhow::{Result, bail};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum FileFormat {
    WindowsPE, // .exe, .dll, .sys
    AppImage,
    DebianPackage, // .deb
    RpmPackage,    // .rpm
    ELF,           // standard linux binary
    Unknown,
}

pub struct SmartDetector;

impl SmartDetector {
    /// Determines the executable format by reading the magic bytes of a file.
    pub fn detect_format<P: AsRef<Path>>(path: P) -> Result<FileFormat> {
        let mut file = File::open(path.as_ref())?;
        
        // Read the first 16 bytes, which is enough for most magic headers
        let mut buffer = [0u8; 16];
        let bytes_read = file.read(&mut buffer)?;
        
        if bytes_read < 4 {
            return Ok(FileFormat::Unknown);
        }

        // Check MZ header for Windows Portable Executable (PE)
        if buffer[0] == 0x4D && buffer[1] == 0x5A { // 'M', 'Z'
            return Ok(FileFormat::WindowsPE);
        }
        
        // Check ELF magic bytes
        if buffer[0] == 0x7F && buffer[1] == 0x45 && buffer[2] == 0x4C && buffer[3] == 0x46 { // \x7F ELF
            // It's an ELF. Let's see if it's an AppImage.
            // AppImages usually have the magic bytes 'A', 'I', 0x02 at offset 8
            if bytes_read >= 11 && buffer[8] == 0x41 && buffer[9] == 0x49 && buffer[10] == 0x02 {
                return Ok(FileFormat::AppImage);
            }
            return Ok(FileFormat::ELF);
        }

        // Check Debian magic bytes (!<arch>\ndebian-binary)
        if buffer.starts_with(b"!<arch>\n") {
            // Note: properly identifying a deb requires reading deeper, but this is a good heuristic
            if path.as_ref().extension().and_then(|s| s.to_str()) == Some("deb") {
                return Ok(FileFormat::DebianPackage);
            }
        }

        // Check RPM magic bytes (ed ab ee db)
        if buffer[0] == 0xED && buffer[1] == 0xAB && buffer[2] == 0xEE && buffer[3] == 0xDB {
            return Ok(FileFormat::RpmPackage);
        }

        Ok(FileFormat::Unknown)
    }
}
