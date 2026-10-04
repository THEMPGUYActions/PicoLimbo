use minecraft_protocol::prelude::{BinaryWriter, BinaryWriterError, EncodePacket, ProtocolVersion};
use pico_nbt::Value;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct Component {
    #[serde(default)]
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub underlined: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub strikethrough: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub obfuscated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub extra: Vec<Component>,
}

const fn is_false(b: &bool) -> bool {
    !*b
}

const LEGACY_COLORS: [(&str, u8, u8, u8); 16] = [
    ("black", 0, 0, 0),
    ("dark_blue", 0, 0, 170),
    ("dark_green", 0, 170, 0),
    ("dark_aqua", 0, 170, 170),
    ("dark_red", 170, 0, 0),
    ("dark_purple", 170, 0, 170),
    ("gold", 255, 170, 0),
    ("gray", 170, 170, 170),
    ("dark_gray", 85, 85, 85),
    ("blue", 85, 85, 255),
    ("green", 85, 255, 85),
    ("aqua", 85, 255, 255),
    ("red", 255, 85, 85),
    ("light_purple", 255, 85, 255),
    ("yellow", 255, 255, 85),
    ("white", 255, 255, 255),
];

fn parse_hex_color(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;

    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

fn closest_legacy_color(color: &str) -> Option<&'static str> {
    let (red, green, blue) = parse_hex_color(color)?;
    let mut closest = None;
    let mut closest_distance = u32::MAX;

    for &(name, legacy_red, legacy_green, legacy_blue) in &LEGACY_COLORS {
        let distance = (red as i32 - legacy_red as i32).pow(2) as u32
            + (green as i32 - legacy_green as i32).pow(2) as u32
            + (blue as i32 - legacy_blue as i32).pow(2) as u32;

        if distance < closest_distance {
            closest_distance = distance;
            closest = Some(name);
        }
    }

    closest
}

fn legacy_color_code(color: &str) -> char {
    match color {
        "black" => '0',
        "dark_blue" => '1',
        "dark_green" => '2',
        "dark_aqua" => '3',
        "dark_red" => '4',
        "dark_purple" => '5',
        "gold" => '6',
        "gray" => '7',
        "dark_gray" => '8',
        "blue" => '9',
        "green" => 'a',
        "aqua" => 'b',
        "red" => 'c',
        "light_purple" => 'd',
        "yellow" => 'e',
        "white" => 'f',
        _ => 'f',
    }
}

impl Component {
    pub fn new<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            text: content.into(),
            ..Default::default()
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn for_protocol(&self, protocol_version: ProtocolVersion) -> Self {
        let mut component = self.clone();

        if protocol_version.is_before_inclusive(ProtocolVersion::V1_15_2) {
            component.color = component.color.as_deref().map(|color| {
                closest_legacy_color(color)
                    .unwrap_or(color)
                    .to_string()
            });
        }

        component.extra = self
            .extra
            .iter()
            .map(|extra| extra.for_protocol(protocol_version))
            .collect();

        component
    }

    pub fn to_nbt(&self) -> Value {
        pico_nbt::to_value(self).unwrap()
    }

    pub fn to_legacy(&self) -> String {
        #[derive(Serialize)]
        struct TextComponent {
            #[serde(default)]
            text: String,
        }
        serde_json::to_string(&TextComponent {
            text: self.to_legacy_impl(true),
        })
        .unwrap_or_default()
    }

    fn to_legacy_impl(&self, is_root: bool) -> String {
        let mut s = String::new();

        if !is_root {
            s.push('§');
            s.push('r');
        }

        if let Some(color) = &self.color {
            let color = closest_legacy_color(color).unwrap_or(color);
            s.push('§');
            s.push(legacy_color_code(color));
        }

        if self.bold {
            s.push('§');
            s.push('l');
        }
        if self.italic {
            s.push('§');
            s.push('o');
        }
        if self.underlined {
            s.push('§');
            s.push('n');
        }
        if self.strikethrough {
            s.push('§');
            s.push('m');
        }
        if self.obfuscated {
            s.push('§');
            s.push('k');
        }

        s.push_str(&self.text);

        for extra in &self.extra {
            s.push_str(&extra.to_legacy_impl(false));
        }

        s
    }
}

impl EncodePacket for Component {
    fn encode(
        &self,
        writer: &mut BinaryWriter,
        protocol_version: ProtocolVersion,
    ) -> Result<(), BinaryWriterError> {
        if protocol_version.is_after_inclusive(ProtocolVersion::V1_20_3) {
            self.to_nbt().encode(writer, protocol_version)?;
        } else {
            self.for_protocol(protocol_version)
                .to_json()
                .encode(writer, protocol_version)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_color_is_preserved_for_1_16_and_newer() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("#ff0088".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_16).color,
            Some("#ff0088".to_string())
        );
    }

    #[test]
    fn test_hex_color_falls_back_for_pre_1_16() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("#ff0088".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_15).color,
            Some("dark_purple".to_string())
        );
    }

    #[test]
    fn test_nested_hex_colors_fall_back_for_pre_1_16() {
        let component = Component {
            extra: vec![Component {
                text: "hello".to_string(),
                color: Some("#ff0088".to_string()),
                ..Component::default()
            }],
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_15).extra[0].color,
            Some("dark_purple".to_string())
        );
    }

    #[test]
    fn test_named_color_is_preserved_for_pre_1_16() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("red".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_15).color,
            Some("red".to_string())
        );
    }

    #[test]
    fn test_hex_color_to_legacy() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("#ff0088".to_string()),
            ..Component::default()
        };

        assert_eq!(component.to_legacy(), r#"{"text":"§5hello"}"#);
    }
}
