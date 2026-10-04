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

fn parse_rgb_channel(value: &str) -> Option<u8> {
    let value = value.trim();

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.trim().parse::<f32>().ok()?;
        if !(0.0..=100.0).contains(&percent) {
            return None;
        }
        return Some((percent * 2.55).round() as u8);
    }

    let value = value.parse::<f32>().ok()?;
    if !(0.0..=255.0).contains(&value) {
        return None;
    }

    Some(value.round() as u8)
}

fn parse_rgb_color(color: &str) -> Option<(u8, u8, u8)> {
    let content = color
        .strip_prefix("rgb(")
        .or_else(|| color.strip_prefix("RGB("))?
        .strip_suffix(')')?;

    let content = content.replace(',', " ");
    let values: Vec<&str> = content.split_whitespace().collect();

    if values.len() != 3 {
        return None;
    }

    Some((
        parse_rgb_channel(values[0])?,
        parse_rgb_channel(values[1])?,
        parse_rgb_channel(values[2])?,
    ))
}

fn parse_hue(value: &str) -> Option<f32> {
    let value = value.trim();

    if let Some(value) = value.strip_suffix("deg") {
        return value.trim().parse::<f32>().ok();
    }
    if let Some(value) = value.strip_suffix("turn") {
        return value.trim().parse::<f32>().ok().map(|turn| turn * 360.0);
    }
    if let Some(value) = value.strip_suffix("rad") {
        return value
            .trim()
            .parse::<f32>()
            .ok()
            .map(|radians| radians.to_degrees());
    }
    if let Some(value) = value.strip_suffix("grad") {
        return value
            .trim()
            .parse::<f32>()
            .ok()
            .map(|gradians| gradians * 0.9);
    }

    value.parse::<f32>().ok()
}

fn parse_percent(value: &str) -> Option<f32> {
    let value = value.trim();
    let value = value.strip_suffix('%').unwrap_or(value);
    let value = value.parse::<f32>().ok()?;
    if !(0.0..=100.0).contains(&value) {
        return None;
    }
    Some(value / 100.0)
}

fn parse_hsl_color(color: &str) -> Option<(u8, u8, u8)> {
    let content = color
        .strip_prefix("hsl(")
        .or_else(|| color.strip_prefix("HSL("))?
        .strip_suffix(')')?;

    let content = content.replace(',', " ");
    let values: Vec<&str> = content.split_whitespace().collect();

    if values.len() != 3 {
        return None;
    }

    let hue = parse_hue(values[0])?.rem_euclid(360.0) / 360.0;
    let saturation = parse_percent(values[1])?;
    let lightness = parse_percent(values[2])?;

    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_sector = hue * 6.0;
    let x = chroma * (1.0 - ((hue_sector % 2.0) - 1.0).abs());

    let (red, green, blue) = match hue_sector {
        h if h < 1.0 => (chroma, x, 0.0),
        h if h < 2.0 => (x, chroma, 0.0),
        h if h < 3.0 => (0.0, chroma, x),
        h if h < 4.0 => (0.0, x, chroma),
        h if h < 5.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let match_value = lightness - chroma / 2.0;
    Some((
        ((red + match_value) * 255.0).round() as u8,
        ((green + match_value) * 255.0).round() as u8,
        ((blue + match_value) * 255.0).round() as u8,
    ))
}

fn parse_hsv_color(color: &str) -> Option<(u8, u8, u8)> {
    let content = color
        .strip_prefix("hsv(")
        .or_else(|| color.strip_prefix("HSV("))
        .or_else(|| color.strip_prefix("hsb("))
        .or_else(|| color.strip_prefix("HSB("))?
        .strip_suffix(')')?;

    let content = content.replace(',', " ");
    let values: Vec<&str> = content.split_whitespace().collect();

    if values.len() != 3 {
        return None;
    }

    let hue = parse_hue(values[0])?.rem_euclid(360.0) / 60.0;
    let saturation = parse_percent(values[1])?;
    let value = parse_percent(values[2])?;

    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue % 2.0) - 1.0).abs());
    let (red, green, blue) = match hue {
        h if h < 1.0 => (chroma, x, 0.0),
        h if h < 2.0 => (x, chroma, 0.0),
        h if h < 3.0 => (0.0, chroma, x),
        h if h < 4.0 => (0.0, x, chroma),
        h if h < 5.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let match_value = value - chroma;
    Some((
        ((red + match_value) * 255.0).round() as u8,
        ((green + match_value) * 255.0).round() as u8,
        ((blue + match_value) * 255.0).round() as u8,
    ))
}

fn parse_color(color: &str) -> Option<(u8, u8, u8)> {
    parse_hex_color(color)
        .or_else(|| parse_rgb_color(color))
        .or_else(|| parse_hsl_color(color))
        .or_else(|| parse_hsv_color(color))
}

pub(crate) fn normalize_color(color: &str) -> Option<String> {
    match color {
        "grey" | "dark_grey" => Some(
            if color == "grey" {
                "gray".to_string()
            } else {
                "dark_gray".to_string()
            },
        ),
        _ => parse_color(color)
            .map(|(red, green, blue)| format!("#{red:02x}{green:02x}{blue:02x}")),
    }
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
        let mut component = self.clone();
        component.normalize_colors();
        serde_json::to_string(&component).unwrap_or_default()
    }

    fn normalize_colors(&mut self) {
        if let Some(color) = &self.color {
            if let Some(normalized) = normalize_color(color) {
                self.color = Some(normalized);
            }
        }

        for extra in &mut self.extra {
            extra.normalize_colors();
        }
    }

    fn for_protocol(&self, protocol_version: ProtocolVersion) -> Self {
        let mut component = self.clone();
        component.normalize_colors();

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
        let mut component = self.clone();
        component.normalize_colors();
        pico_nbt::to_value(&component).unwrap()
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
            let normalized = normalize_color(color);
            let color = normalized.as_deref().unwrap_or(color);
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
    fn test_rgb_color_is_normalized() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("rgb(255, 0, 128)".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_16).color,
            Some("#ff0080".to_string())
        );
    }

    #[test]
    fn test_hsl_color_is_normalized() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("hsl(330, 100%, 50%)".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_16).color,
            Some("#ff0080".to_string())
        );
    }

    #[test]
    fn test_hsv_color_is_normalized() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("hsv(330, 100%, 100%)".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_16).color,
            Some("#ff0080".to_string())
        );
    }

    #[test]
    fn test_legacy_aliases_are_normalized() {
        let component = Component {
            text: "hello".to_string(),
            color: Some("grey".to_string()),
            ..Component::default()
        };

        assert_eq!(
            component.for_protocol(ProtocolVersion::V1_16).color,
            Some("gray".to_string())
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
