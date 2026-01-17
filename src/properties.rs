use std::{fmt, fs, io::Read};

pub struct Properties {
    pub article_file: String,
    pub category_file: String,
    pub article_separator: u8,
    pub category_list_boundary: u8,
    pub category_entry_separator: u8,
}
impl fmt::Debug for Properties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper closure to format bytes based on your rule
        let format_byte = |b: u8| -> String {
            if b.is_ascii_control() {
                format!("0x{:02X}", b) // Hex for control chars
            } else {
                format!("{}", b as char) // Char for "proper" chars
            }
        };

        f.debug_struct("Properties")
            .field("article_file", &self.article_file)
            .field("category_file", &self.category_file)
            .field("article_separator", &format_byte(self.article_separator))
            .field("category_list_boundary", &format_byte(self.category_list_boundary))
            .field("category_entry_separator", &format_byte(self.category_entry_separator))
            .finish()
    }
}

const CONFIG_SIZE_LIMIT: usize = 16384;

impl Properties {
    // The return type is quite nice. We build enums of our own to return the intended errors
    pub fn try_from_config_file(filename: &str) -> Result<Properties, ConfigError> {
        // we use ./config.ini by default but users can insert a parameter to add their own
        let file_info = {
            let file = fs::File::open(filename).map_err(ConfigError::IoError)?;

            // Set limit: 1MB
            let mut handle = file.take(CONFIG_SIZE_LIMIT as u64);
            let mut buffer = String::new();

            match handle.read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(err) => return Err(ConfigError::IoError(err)),
            }
        };
        let file_info_str = &file_info[..];
        let mut prop_builder = PropertiesBuilder::default();
        for el in file_info_str
            .lines()
            .map(|x| x.trim())
            .filter(|x| !x.is_empty() && !x.starts_with(";"))
        {
            let Some((key, value)) = el.split_once("=") else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());

            match key {
                // I think I have cleared this configuration part incredibly well ngl
                "article_file" => {
                    // A lot of this trimming logic seems reusable but idk.
                    // This part doesn't need to be THAT good either way
                    let value = value.trim_matches(&['\'', '"']);
                    if value != "" {
                        prop_builder.article_file(value.to_string())
                    }
                }
                "category_file" => {
                    let value = value.trim_matches(&['\'', '"']);
                    if value != "" {
                        prop_builder.category_file(value.to_string())
                    }
                }
                "article_separator" => {
                    if let Some(val) = str_to_delimiter(value) {
                        prop_builder.article_separator(val);
                    }
                }
                "category_list_boundary" => {
                    if let Some(val) = str_to_delimiter(value) {
                        prop_builder.category_list_boundary(val);
                    }
                }
                "category_entry_separator" => {
                    if let Some(val) = str_to_delimiter(value) {
                        prop_builder.category_entry_separator(val);
                    }
                }
                _ => {}
            }
        }
        prop_builder.build()
    }
}

fn str_to_delimiter(val: &str) -> Option<u8> {
    if val.starts_with("0x") {
        // Hexadecimal values
        return u8::from_str_radix(val.strip_prefix("0x").unwrap(), 16).ok();
    } else if val.len() == 1 {
        return val.bytes().nth(0);
    } else if val.starts_with("'"){
        return val.bytes().nth(1);
    } else {
        return None;
    }
}

#[derive(Default)]
pub struct PropertiesBuilder {
    article_file: Option<String>,
    category_file: Option<String>,
    article_separator: Option<u8>,
    category_list_boundary: Option<u8>,
    category_entry_separator: Option<u8>,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingFields(Vec<&'static str>),
    IoError(std::io::Error),
}

impl PropertiesBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn article_file(&mut self, path: String) {
        self.article_file = Some(path);
    }

    pub fn category_file(&mut self, path: String) {
        self.category_file = Some(path);
    }

    pub fn article_separator(&mut self, sep: u8) {
        self.article_separator = Some(sep);
    }

    pub fn category_list_boundary(&mut self, bound: u8) {
        self.category_list_boundary = Some(bound);
    }

    pub fn category_entry_separator(&mut self, sep: u8) {
        self.category_entry_separator = Some(sep);
    }

    pub fn build(self) -> Result<Properties, ConfigError> {
        let mut missing = Vec::new();

        if self.article_file.is_none() {
            missing.push("article_file");
        }
        if self.category_file.is_none() {
            missing.push("category_file");
        }

        if !missing.is_empty() {
            return Err(ConfigError::MissingFields(missing));
        }

        Ok(Properties {
            article_file: self.article_file.unwrap_or_default(),
            category_file: self.category_file.unwrap_or_default(),
            article_separator: self.article_separator.unwrap_or(0x1E), // Default value
            category_list_boundary: self.category_list_boundary.unwrap_or(b','),
            category_entry_separator: self.category_entry_separator.unwrap_or(b'\n'),
        })
    }
}
