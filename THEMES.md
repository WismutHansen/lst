# Theming System

This project uses the [Tinted Theming](https://github.com/tinted-theming/home) approach for consistent color schemes across all applications. Our theming system supports both standalone usage and integration with [Tinty](https://github.com/tinted-theming/tinty) for centralized theme management.

## Overview

The theming system is built around the [Base16](https://github.com/tinted-theming/home/blob/main/styling.md) and [Base24](https://github.com/tinted-theming/base24/blob/master/styling.md) color specifications, providing:

- **250+ color schemes** from the tinted-theming ecosystem
- **Consistent theming** across desktop, mobile, and CLI applications
- **Runtime theme switching** without application restart
- **Optional Tinty integration** for centralized theme management
- **Semantic color mapping** for maintainable theme definitions

## Configuration Structure

### Basic Theme Configuration

Add a `[theme]` section to your `config.toml`:

```toml
[theme]
# Theme system (base16, base24, or custom)
system = "base16"

# Current active scheme name
scheme = "catppuccin-mocha"

# Base16/Base24 color palette
[theme.palette]
base00 = "#1e1e2e"  # Default Background
base01 = "#181825"  # Lighter Background (status bars, line numbers)
base02 = "#313244"  # Selection Background
base03 = "#45475a"  # Comments, Invisibles, Line Highlighting
base04 = "#585b70"  # Dark Foreground (status bars)
base05 = "#cdd6f4"  # Default Foreground, Caret, Delimiters, Operators
base06 = "#f5e0dc"  # Light Foreground (not often used)
base07 = "#b4befe"  # Light Background (not often used)
base08 = "#f38ba8"  # Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
base09 = "#fab387"  # Integers, Boolean, Constants, XML Attributes, Markup Link Url
base0A = "#f9e2af"  # Classes, Markup Bold, Search Text Background
base0B = "#a6e3a1"  # Strings, Inherited Class, Markup Code, Diff Inserted
base0C = "#94e2d5"  # Support, Regular Expressions, Escape Characters, Markup Quotes
base0D = "#89b4fa"  # Functions, Methods, Attribute IDs, Headings
base0E = "#cba6f7"  # Keywords, Storage, Selector, Markup Italic, Diff Changed
base0F = "#f2cdcd"  # Deprecated, Opening/Closing Embedded Language Tags

# Semantic color mappings (maps base16 colors to app-specific purposes)
[theme.semantic]
background = "base00"
foreground = "base05"
primary = "base0D"
secondary = "base06"
accent = "base0E"
muted = "base03"
border = "base02"
success = "base0B"
warning = "base0A"
error = "base08"
info = "base0C"
```

### Advanced Configuration

```toml
[theme]
system = "base16"
scheme = "catppuccin-mocha"

# Theme inheritance (optional)
inherits = "base16-default"

# Light/dark mode support
[theme.variants]
light = "catppuccin-latte"
dark = "catppuccin-mocha"

# Custom overrides (optional)
[theme.overrides]
# Override specific palette colors
base08 = "#ff6b6b"  # Custom red

# Override semantic mappings
[theme.overrides.semantic]
error = "base09"  # Use orange instead of red for errors
```

## Base16 Color Specification

The Base16 specification defines 16 colors with specific semantic meanings:

| Color   | Purpose | Example Usage |
|---------|---------|---------------|
| `base00` | Default Background | Main background color |
| `base01` | Lighter Background | Status bars, line numbers |
| `base02` | Selection Background | Selected text, highlighted lines |
| `base03` | Comments | Comments, disabled text |
| `base04` | Dark Foreground | Status bar text |
| `base05` | Default Foreground | Main text color |
| `base06` | Light Foreground | Light text (rarely used) |
| `base07` | Light Background | Light backgrounds (rarely used) |
| `base08` | Red | Variables, errors, deletion |
| `base09` | Orange | Integers, constants, warnings |
| `base0A` | Yellow | Classes, search highlights |
| `base0B` | Green | Strings, additions, success |
| `base0C` | Cyan | Support functions, info |
| `base0D` | Blue | Functions, links, primary actions |
| `base0E` | Purple | Keywords, storage, accent |
| `base0F` | Brown | Deprecated, special |

## Theme Files

### Standalone Theme Files

Create theme files in `~/.config/[app-name]/themes/`:

```toml
# ~/.config/lst/themes/my-custom-theme.toml
[theme]
system = "base16"
scheme = "my-custom-theme"

[theme.palette]
base00 = "#1a1a1a"
base01 = "#2a2a2a"
# ... etc

[theme.semantic]
background = "base00"
foreground = "base05"
# ... etc
```

### Built-in Themes

The application ships with several built-in themes:

- `base16-default-dark` - Default dark theme
- `base16-default-light` - Default light theme  
- `base16-terminal` - Uses terminal colors (for terminal compatibility)
- `catppuccin-mocha` - Popular dark theme
- `catppuccin-latte` - Popular light theme

## Tinty Integration

[Tinty](https://github.com/tinted-theming/tinty) is an optional theme manager that provides centralized theming across multiple applications.

### Setup with Tinty

1. **Install Tinty:**
   ```bash
   cargo install tinty
   # or
   brew install tinty
   ```

2. **Configure Tinty** (`~/.config/tinted-theming/tinty/config.toml`):
   ```toml
   [[items]]
   name = "lst"
   path = "https://github.com/tinted-theming/base16-template"  # Replace with actual template
   themes-dir = "themes"
   hook = "cp -f \"$TINTY_THEME_FILE_PATH\" ~/.config/lst/current-theme.toml && pkill -USR1 lst"
   supported-systems = ["base16", "base24"]
   ```

3. **Sync and apply themes:**
   ```bash
   tinty sync
   tinty apply base16-catppuccin-mocha
   ```

### Tinty Environment Variables

When using Tinty hooks, the following environment variables are available:

| Variable | Description | Example |
|----------|-------------|---------|
| `TINTY_THEME_FILE_PATH` | Path to generated theme file | `/home/user/.local/share/tinted-theming/tinty/lst.toml` |
| `TINTY_SCHEME_ID` | Full scheme identifier | `base16-catppuccin-mocha` |
| `TINTY_SCHEME_SYSTEM` | Theme system | `base16` or `base24` |
| `TINTY_SCHEME_SLUG` | Scheme name | `catppuccin-mocha` |
| `TINTY_SCHEME_VARIANT` | Light or dark | `light` or `dark` |
| `TINTY_SCHEME_PALETTE_BASE00_HEX_R` | Red component of base00 | `1e` |
| `TINTY_SCHEME_PALETTE_BASE00_RGB_R` | Red component (0-255) | `30` |

## Implementation Guide

### For Application Developers

1. **Theme Loading:**
   ```rust
   // Load theme from config
   let theme = config.theme.unwrap_or_default();
   
   // Generate CSS custom properties
   let css_vars = generate_css_variables(&theme);
   
   // Apply to application
   apply_theme_variables(css_vars);
   ```

2. **Runtime Theme Switching:**
   ```rust
   // Watch for theme file changes
   let watcher = notify::watcher(|event| {
       if let Ok(event) = event {
           if event.path.ends_with("current-theme.toml") {
               reload_theme();
           }
       }
   });
   ```

3. **CSS Variable Generation:**
   ```rust
   fn generate_css_variables(theme: &Theme) -> String {
       let mut css = String::new();
       
       // Generate palette variables
       for (name, color) in &theme.palette {
           css.push_str(&format!("--color-{}: {};\n", name, color));
       }
       
       // Generate semantic variables
       for (name, base_color) in &theme.semantic {
           let color = theme.palette.get(base_color).unwrap();
           css.push_str(&format!("--color-{}: {};\n", name, color));
       }
       
       css
   }
   ```

### For Frontend Applications

1. **CSS Usage:**
   ```css
   :root {
     /* Palette colors */
     --color-base00: #1e1e2e;
     --color-base01: #181825;
     /* ... */
     
     /* Semantic colors */
     --color-background: var(--color-base00);
     --color-foreground: var(--color-base05);
     --color-primary: var(--color-base0D);
   }
   
   .main-content {
     background-color: var(--color-background);
     color: var(--color-foreground);
   }
   
   .primary-button {
     background-color: var(--color-primary);
     color: var(--color-background);
   }
   ```

2. **React/TypeScript Integration:**
   ```typescript
   interface ThemeColors {
     background: string;
     foreground: string;
     primary: string;
     // ... etc
   }
   
   const ThemeContext = createContext<ThemeColors | null>(null);
   
   export const useTheme = () => {
     const theme = useContext(ThemeContext);
     if (!theme) throw new Error('useTheme must be used within ThemeProvider');
     return theme;
   };
   ```

## Theme Creation

### Creating Custom Themes

1. **Start with a base16 template:**
   ```toml
   [theme]
   system = "base16"
   scheme = "my-theme"
   
   [theme.palette]
   # Define your 16 colors following base16 guidelines
   base00 = "#your-background"
   base05 = "#your-foreground"
   # ... etc
   
   [theme.semantic]
   # Map semantic names to base16 colors
   background = "base00"
   foreground = "base05"
   # ... etc
   ```

2. **Test your theme:**
   ```bash
   # Copy to themes directory
   cp my-theme.toml ~/.config/lst/themes/
   
   # Apply theme
   lst config set theme.scheme my-theme
   ```

3. **Share your theme:**
   - Submit to [tinted-theming/schemes](https://github.com/tinted-theming/schemes)
   - Create a template repository for your application
   - Share theme files directly

### Theme Guidelines

- **Follow base16 semantics** - Use colors for their intended purposes
- **Test in both light and dark environments**
- **Ensure sufficient contrast** for accessibility
- **Provide both light and dark variants** when possible
- **Use meaningful scheme names** that reflect the theme's character

## Migration Guide

### From Hardcoded Colors

1. **Identify current colors** in your application
2. **Map to base16 semantics** - determine which base16 color each represents
3. **Create semantic mappings** in your theme configuration
4. **Replace hardcoded values** with CSS variables or theme references
5. **Test with multiple themes** to ensure consistency

### From Other Theme Systems

1. **Convert color palettes** to base16 format
2. **Map semantic meanings** to base16 colors
3. **Update configuration format** to match our theme structure
4. **Test theme switching** functionality

## Troubleshooting

### Common Issues

1. **Theme not loading:**
   - Check file path and permissions
   - Validate TOML syntax
   - Ensure all required colors are defined

2. **Colors not updating:**
   - Verify CSS variable generation
   - Check for cached styles
   - Ensure theme reload mechanism is working

3. **Tinty integration issues:**
   - Verify Tinty configuration
   - Check hook execution permissions
   - Test environment variable availability

### Debug Commands

```bash
# List available themes
lst themes list

# Show current theme
lst themes current

# Validate theme file
lst themes validate path/to/theme.toml

# Test theme application
lst themes apply theme-name --dry-run
```

## Resources

- [Tinted Theming Home](https://github.com/tinted-theming/home) - Main project documentation
- [Base16 Specification](https://github.com/tinted-theming/home/blob/main/styling.md) - Color specification
- [Base24 Specification](https://github.com/tinted-theming/base24/blob/master/styling.md) - Extended color specification
- [Tinty](https://github.com/tinted-theming/tinty) - Universal theme manager
- [Theme Gallery](https://tinted-theming.github.io/tinted-gallery) - Browse available themes
- [Scheme Repository](https://github.com/tinted-theming/schemes) - Official color schemes

## Contributing

When contributing themes or theme-related features:

1. **Follow the base16 specification** for color semantics
2. **Test with multiple themes** to ensure compatibility
3. **Document any new semantic mappings**
4. **Provide both light and dark variants** when possible
5. **Consider accessibility** and contrast requirements

This theming system provides a solid foundation for consistent, customizable, and maintainable color schemes across all your applications.