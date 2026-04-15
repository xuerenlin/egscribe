use encoding_rs::{Encoding, UTF_8, GBK, GB18030, BIG5, EUC_JP, EUC_KR, SHIFT_JIS, WINDOWS_1252};
use chardetng::EncodingDetector;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Supported charset enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Charset {
    UTF8,
    GBK,
    GB18030,
    Big5,
    EUCJP,
    EUCKR,
    ShiftJIS,
    Windows1252,
    Unknown,
}

impl Charset {
    /// Convert from string to charset enumeration
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "utf-8" | "utf8" => Charset::UTF8,
            "gbk" => Charset::GBK,
            "gb18030" => Charset::GB18030,
            "big5" => Charset::Big5,
            "euc-jp" | "eucjp" => Charset::EUCJP,
            "euc-kr" | "euckr" => Charset::EUCKR,
            "shift-jis" | "shiftjis" | "sjis" => Charset::ShiftJIS,
            "windows-1252" | "windows1252" | "cp1252" => Charset::Windows1252,
            _ => Charset::Unknown,
        }
    }

    /// Get charset display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Charset::UTF8 => "UTF-8",
            Charset::GBK => "GBK",
            Charset::GB18030 => "GB18030",
            Charset::Big5 => "Big5",
            Charset::EUCJP => "EUC-JP",
            Charset::EUCKR => "EUC-KR",
            Charset::ShiftJIS => "Shift_JIS",
            Charset::Windows1252 => "Windows-1252",
            Charset::Unknown => "Unknown",
        }
    }

    /// Get corresponding encoding_rs encoding
    fn to_encoding(&self) -> &'static Encoding {
        match self {
            Charset::UTF8 => UTF_8,
            Charset::GBK => GBK,
            Charset::GB18030 => GB18030,
            Charset::Big5 => BIG5,
            Charset::EUCJP => EUC_JP,
            Charset::EUCKR => EUC_KR,
            Charset::ShiftJIS => SHIFT_JIS,
            Charset::Windows1252 => WINDOWS_1252,
            Charset::Unknown => UTF_8, // Default to UTF-8
        }
    }
}

/// Line ending format enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LineEnding {
    LF,     // Unix/Linux: \n
    CRLF,   // Windows/DOS: \r\n
    CR,     // Classic Mac: \r (deprecated)
}

#[allow(dead_code)]
impl LineEnding {
    /// Convert from string to line ending format enumeration
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lf" | "unix" | "linux" => LineEnding::LF,
            "crlf" | "dos" | "windows" => LineEnding::CRLF,
            "cr" | "mac" => LineEnding::CR,
            _ => LineEnding::LF, // Default to LF
        }
    }

    /// Get line ending format display name
    pub fn display_name(&self) -> &'static str {
        match self {
            LineEnding::LF => "LF (Unix)",
            LineEnding::CRLF => "CRLF (Dos)",
            LineEnding::CR => "CR (Mac)",
        }
    }

    /// Get line ending string
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LF => "\n",
            LineEnding::CRLF => "\r\n",
            LineEnding::CR => "\r",
        }
    }

    /// Detect line ending format in text
    pub fn detect_from_text(text: &str) -> Self {
        if text.contains("\r\n") {
            LineEnding::CRLF
        } else if text.contains('\r') {
            LineEnding::CR
        } else {
            LineEnding::LF
        }
    }

    /// Convert text to specified line ending format
    pub fn convert_text(&self, text: &str) -> String {
        // First normalize to LF
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        
        // Then convert to target format
        match self {
            LineEnding::LF => normalized,
            LineEnding::CRLF => normalized.replace('\n', "\r\n"),
            LineEnding::CR => normalized.replace('\n', "\r"),
        }
    }
}

/// File encoding information
#[derive(Debug, Clone)]
pub struct FileEncoding {
    pub charset: Charset,
    pub has_bom: bool,
    pub line_ending: LineEnding,
}

impl FileEncoding {
    pub fn new(charset: Charset, has_bom: bool, line_ending: LineEnding) -> Self {
        Self {
            charset,
            has_bom,
            line_ending,
        }
    }
}

/// Charset manager
pub struct EncodingManager {
    file_encodings: HashMap<String, FileEncoding>,
}

impl EncodingManager {
    pub fn new() -> Self {
        Self {
            file_encodings: HashMap::new(),
        }
    }

    /// Detect file encoding
    pub fn detect_file_encoding(&mut self, file_path: &str) -> Result<FileEncoding, std::io::Error> {
        // Check cache
        if let Some(cached) = self.file_encodings.get(file_path) {
            return Ok(cached.clone());
        }

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ));
        }

        // Read file content
        let content = fs::read(file_path)?;
        if content.is_empty() {
            let encoding = FileEncoding::new(Charset::UTF8, false, LineEnding::LF);
            self.file_encodings.insert(file_path.to_string(), encoding.clone());
            return Ok(encoding);
        }

        // Detect BOM
        let has_bom = self.detect_bom(&content);
        
        // Use chardetng to detect encoding
        let mut detector = EncodingDetector::new();
        detector.feed(&content, true);
        let encoding = detector.guess(None, true);

        // Convert to our charset enumeration
        let charset = self.encoding_to_charset(&encoding);
        
        // Detect line ending format
        let line_ending = if let Ok(text) = String::from_utf8(content.clone()) {
            LineEnding::detect_from_text(&text)
        } else {
            // If cannot decode as UTF-8, try to detect line endings in raw bytes
            if content.windows(2).any(|w| w == b"\r\n") {
                LineEnding::CRLF
            } else if content.contains(&b'\r') {
                LineEnding::CR
            } else {
                LineEnding::LF
            }
        };
        
        let file_encoding = FileEncoding::new(charset, has_bom, line_ending);
        
        // Cache result
        self.file_encodings.insert(file_path.to_string(), file_encoding.clone());
        
        Ok(file_encoding)
    }

    /// Detect BOM (Byte Order Mark)
    fn detect_bom(&self, content: &[u8]) -> bool {
        if content.len() >= 3 {
            // UTF-8 BOM
            if content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF {
                return true;
            }
        }
        if content.len() >= 2 {
            // UTF-16 LE BOM
            if content[0] == 0xFF && content[1] == 0xFE {
                return true;
            }
            // UTF-16 BE BOM
            if content[0] == 0xFE && content[1] == 0xFF {
                return true;
            }
        }
        false
    }

    /// Convert encoding_rs encoding to our charset enumeration
    fn encoding_to_charset(&self, encoding: &Encoding) -> Charset {
        if encoding == UTF_8 {
            Charset::UTF8
        } else if encoding == GBK {
            Charset::GBK
        } else if encoding == GB18030 {
            Charset::GB18030
        } else if encoding == BIG5 {
            Charset::Big5
        } else if encoding == EUC_JP {
            Charset::EUCJP
        } else if encoding == EUC_KR {
            Charset::EUCKR
        } else if encoding == SHIFT_JIS {
            Charset::ShiftJIS
        } else if encoding == WINDOWS_1252 {
            Charset::Windows1252
        } else {
            Charset::Unknown
        }
    }

    /// Read file with specified charset
    pub fn read_file_with_charset(&mut self, file_path: &str, charset: &Charset) -> Result<String, std::io::Error> {
        let content = fs::read(file_path)?;
        
        if content.is_empty() {
            return Ok(String::new());
        }

        // Remove BOM if present
        let content_without_bom = if self.detect_bom(&content) {
            if content.len() >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF {
                &content[3..]
            } else {
                &content
            }
        } else {
            &content
        };

        // Decode with specified encoding
        let encoding = charset.to_encoding();
        let (decoded, _, had_errors) = encoding.decode(content_without_bom);
        
        if had_errors {
            log::warn!("Encoding errors detected when reading file with charset {}: {}", 
                      charset.display_name(), file_path);
        }

        Ok(decoded.into_owned())
    }

    /// Read file and convert to UTF-8
    pub fn read_file_as_utf8(&mut self, file_path: &str) -> Result<String, std::io::Error> {
        let encoding_info = self.detect_file_encoding(file_path)?;
        let content = fs::read(file_path)?;
        
        if content.is_empty() {
            return Ok(String::new());
        }

        // Remove BOM if present
        let content_without_bom = if encoding_info.has_bom {
            if content.len() >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF {
                &content[3..]
            } else {
                &content
            }
        } else {
            &content
        };

        // Decode to UTF-8
        let encoding = encoding_info.charset.to_encoding();
        let (decoded, _, had_errors) = encoding.decode(content_without_bom);
        
        if had_errors {
            log::warn!("Encoding errors detected when reading file: {}", file_path);
        }

        // Normalize line endings to LF for internal processing
        let text = decoded.into_owned();
        let normalized_text = text.replace("\r\n", "\n").replace('\r', "\n");

        Ok(normalized_text)
    }

    /// Save UTF-8 content with specified encoding
    pub fn write_file_with_encoding(
        &self,
        file_path: &str,
        content: &str,
        charset: &Charset,
        line_ending: &LineEnding,
    ) -> Result<(), std::io::Error> {
        // Convert line ending format
        let converted_content = line_ending.convert_text(content);
        
        let encoding = charset.to_encoding();
        let (encoded, _, had_errors) = encoding.encode(&converted_content);
        
        if had_errors {
            log::warn!("Encoding errors detected when writing file: {}", file_path);
        }

        // Add BOM if needed
        let final_content = if charset == &Charset::UTF8 {
            let mut with_bom = Vec::new();
            with_bom.extend_from_slice(&[0xEF, 0xBB, 0xBF]); // UTF-8 BOM
            with_bom.extend_from_slice(&encoded);
            with_bom
        } else {
            encoded.into_owned()
        };

        fs::write(file_path, final_content)
    }

    /// Get file encoding information
    pub fn get_file_encoding(&self, file_path: &str) -> Option<&FileEncoding> {
        self.file_encodings.get(file_path)
    }

    /// Clear file encoding cache
    #[allow(dead_code)]
    pub fn clear_cache(&mut self) {
        self.file_encodings.clear();
    }

    /// Get all supported charsets
    pub fn get_supported_charsets() -> Vec<Charset> {
        vec![
            Charset::UTF8,
            Charset::GBK,
            Charset::GB18030,
            Charset::Big5,
            Charset::EUCJP,
            Charset::EUCKR,
            Charset::ShiftJIS,
            Charset::Windows1252,
        ]
    }

    /// Set file encoding (for manual specification)
    pub fn set_file_encoding(&mut self, file_path: &str, charset: Charset, has_bom: bool) {
        let file_encoding = FileEncoding::new(charset, has_bom, LineEnding::LF);
        self.file_encodings.insert(file_path.to_string(), file_encoding);
    }

    /// Set file encoding and line ending format (for manual specification)
    pub fn set_file_encoding_with_line_ending(&mut self, file_path: &str, charset: Charset, has_bom: bool, line_ending: LineEnding) {
        let file_encoding = FileEncoding::new(charset, has_bom, line_ending);
        self.file_encodings.insert(file_path.to_string(), file_encoding);
    }

    /// Get all supported line ending formats
    pub fn get_supported_line_endings() -> Vec<LineEnding> {
        vec![
            LineEnding::LF,
            LineEnding::CRLF,
            LineEnding::CR,
        ]
    }
}

impl Default for EncodingManager {
    fn default() -> Self {
        Self::new()
    }
} 