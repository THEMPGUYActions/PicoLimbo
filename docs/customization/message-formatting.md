# Message Formatting

PicoLimbo supports **MiniMessage** formatting for styling text messages displayed to players. MiniMessage provides a modern, readable syntax for text formatting that's more intuitive than legacy color codes.

## What is MiniMessage?

MiniMessage is a text formatting system that uses XML-like tags to apply colors and formatting to text. Instead of cryptic codes like `§a` or `&c`, you can use descriptive tags like `<green>` or `<red>`.
Learn more about MiniMessage on [Adventure's documentation](https://docs.advntr.dev/minimessage/index.html).

> [!NOTE]
> MiniMessage is the recommended formatting method. Legacy color codes (like `§a` or `&c`) are still supported but may be deprecated in future versions.

## Basic Syntax

MiniMessage uses angle brackets `<>` to define formatting tags:

:::code-group
```xml
<red>This text is red</red>
<green>This text is green</green>
<bold>This text is bold</bold>
```
:::

## Supported Features

PicoLimbo currently supports a **subset** of MiniMessage features:

### ✅ Supported
- **Colors** - All standard Minecraft colors, hex colors, and the MiniMessage verbose color form
- **Formatting** - `<bold>`, `<italic>`, `<underlined>`, `<strikethrough>` and `<obfuscated>`
- **New lines** - `<newline>`
- **Additional color formats** - PicoLimbo also accepts RGB, HSL, and HSV/HSB functional color values and converts them to RGB

### ❌ Not Yet Supported
- Gradients
- Hover events
- Click events
- Custom fonts
- Keybinds
- Translatable components

## Hex Colors

Hex colors use the MiniMessage format `<#RRGGBB>` and are supported by Minecraft 1.16 and newer. For older clients, PicoLimbo automatically falls back to the closest legacy Minecraft color.

The verbose MiniMessage color syntax is also supported:
`<color:#RRGGBB>`, `<colour:#RRGGBB>`, and `<c:#RRGGBB>`.

PicoLimbo additionally accepts RGB, HSL, and HSV/HSB color values. These are PicoLimbo extensions rather than standard MiniMessage tags. In MiniMessage tags they use a colon-separated form so they remain compatible with PicoLimbo's XML-based parser: `<rgb:R:G:B>`, `<hsl:H:S:L>`, and `<hsv:H:S:V>`. HSL/HSV saturation, lightness, and value are percentages from 0 to 100. The values are converted to an RGB hex color before the component is sent to the client. Alpha/transparency is not represented because the current text component model stores text colors as RGB.

:::code-group
```xml
<rgb:255:0:128>RGB</rgb:255:0:128>
<hsl:330:100:50>HSL</hsl:330:100:50>
<hsv:330:100:100>HSV</hsv:330:100:100>
<color:#ff0088>Verbose hex</color:#ff0088>
```
:::

:::code-group
```toml [server.toml]
welcome_message = "<#ff0088>Welcome to <#8b5cf6>PicoLimbo</#8b5cf6>!</#ff0088>"
```
:::

## Examples

:::code-group
```toml [server.toml]
welcome_message = "<green>Welcome to <bold>PicoLimbo</bold>!</green>"
```
:::
