use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg(feature = "tauri")]
use specta::Type;

/// Theme system type (base16, base24, or custom)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "tauri", derive(Type))]
#[serde(rename_all = "lowercase")]
pub enum ThemeSystem {
    Base16,
    Base24,
    Custom,
}

impl Default for ThemeSystem {
    fn default() -> Self {
        Self::Base16
    }
}

/// Theme variant (light or dark)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "tauri", derive(Type))]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    Light,
    Dark,
}

/// Color palette containing base16/base24 colors
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ThemePalette {
    // Base16 colors (required)
    pub base00: Option<String>, // Default Background
    pub base01: Option<String>, // Lighter Background
    pub base02: Option<String>, // Selection Background
    pub base03: Option<String>, // Comments, Invisibles
    pub base04: Option<String>, // Dark Foreground
    pub base05: Option<String>, // Default Foreground
    pub base06: Option<String>, // Light Foreground
    pub base07: Option<String>, // Light Background
    pub base08: Option<String>, // Red
    pub base09: Option<String>, // Orange
    #[serde(rename = "base0A")]
    pub base0a: Option<String>, // Yellow
    #[serde(rename = "base0B")]
    pub base0b: Option<String>, // Green
    #[serde(rename = "base0C")]
    pub base0c: Option<String>, // Cyan
    #[serde(rename = "base0D")]
    pub base0d: Option<String>, // Blue
    #[serde(rename = "base0E")]
    pub base0e: Option<String>, // Purple
    #[serde(rename = "base0F")]
    pub base0f: Option<String>, // Brown

    // Base24 additional colors (optional)
    pub base10: Option<String>,
    pub base11: Option<String>,
    pub base12: Option<String>,
    pub base13: Option<String>,
    pub base14: Option<String>,
    pub base15: Option<String>,
    pub base16: Option<String>,
    pub base17: Option<String>,
}

/// Semantic color mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct SemanticColors {
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_foreground")]
    pub foreground: String,
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_secondary")]
    pub secondary: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_muted")]
    pub muted: String,
    #[serde(default = "default_border")]
    pub border: String,
    #[serde(default = "default_success")]
    pub success: String,
    #[serde(default = "default_warning")]
    pub warning: String,
    #[serde(default = "default_error")]
    pub error: String,
    #[serde(default = "default_info")]
    pub info: String,
}

impl Default for SemanticColors {
    fn default() -> Self {
        Self {
            background: default_background(),
            foreground: default_foreground(),
            primary: default_primary(),
            secondary: default_secondary(),
            accent: default_accent(),
            muted: default_muted(),
            border: default_border(),
            success: default_success(),
            warning: default_warning(),
            error: default_error(),
            info: default_info(),
        }
    }
}

/// Theme variant configuration for light/dark mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ThemeVariants {
    pub light: Option<String>,
    pub dark: Option<String>,
}

/// Theme overrides for customization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ThemeOverrides {
    #[serde(flatten)]
    pub palette: BTreeMap<String, String>,
    pub semantic: Option<BTreeMap<String, String>>,
}

/// Complete theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct Theme {
    #[serde(default)]
    pub system: ThemeSystem,
    #[serde(default)]
    pub scheme: String,
    #[serde(default)]
    pub palette: ThemePalette,
    #[serde(default)]
    pub semantic: SemanticColors,
    pub inherits: Option<String>,
    pub variants: Option<ThemeVariants>,
    pub overrides: Option<ThemeOverrides>,

    // Metadata
    pub name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub variant: Option<ThemeVariant>,
}

impl Theme {
    /// Generate CSS custom properties from the theme
    pub fn generate_css_variables(&self) -> String {
        let mut css = String::new();

        // Generate palette variables
        css.push_str("  /* Base16/Base24 Palette Colors */\n");
        if let Some(ref color) = self.palette.base00 {
            css.push_str(&format!("  --color-base00: {};\n", color));
        }
        if let Some(ref color) = self.palette.base01 {
            css.push_str(&format!("  --color-base01: {};\n", color));
        }
        if let Some(ref color) = self.palette.base02 {
            css.push_str(&format!("  --color-base02: {};\n", color));
        }
        if let Some(ref color) = self.palette.base03 {
            css.push_str(&format!("  --color-base03: {};\n", color));
        }
        if let Some(ref color) = self.palette.base04 {
            css.push_str(&format!("  --color-base04: {};\n", color));
        }
        if let Some(ref color) = self.palette.base05 {
            css.push_str(&format!("  --color-base05: {};\n", color));
        }
        if let Some(ref color) = self.palette.base06 {
            css.push_str(&format!("  --color-base06: {};\n", color));
        }
        if let Some(ref color) = self.palette.base07 {
            css.push_str(&format!("  --color-base07: {};\n", color));
        }
        if let Some(ref color) = self.palette.base08 {
            css.push_str(&format!("  --color-base08: {};\n", color));
        }
        if let Some(ref color) = self.palette.base09 {
            css.push_str(&format!("  --color-base09: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0a {
            css.push_str(&format!("  --color-base0A: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0b {
            css.push_str(&format!("  --color-base0B: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0c {
            css.push_str(&format!("  --color-base0C: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0d {
            css.push_str(&format!("  --color-base0D: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0e {
            css.push_str(&format!("  --color-base0E: {};\n", color));
        }
        if let Some(ref color) = self.palette.base0f {
            css.push_str(&format!("  --color-base0F: {};\n", color));
        }

        // Base24 colors if present
        if let Some(ref color) = self.palette.base10 {
            css.push_str(&format!("  --color-base10: {};\n", color));
        }
        if let Some(ref color) = self.palette.base11 {
            css.push_str(&format!("  --color-base11: {};\n", color));
        }
        if let Some(ref color) = self.palette.base12 {
            css.push_str(&format!("  --color-base12: {};\n", color));
        }
        if let Some(ref color) = self.palette.base13 {
            css.push_str(&format!("  --color-base13: {};\n", color));
        }
        if let Some(ref color) = self.palette.base14 {
            css.push_str(&format!("  --color-base14: {};\n", color));
        }
        if let Some(ref color) = self.palette.base15 {
            css.push_str(&format!("  --color-base15: {};\n", color));
        }
        if let Some(ref color) = self.palette.base16 {
            css.push_str(&format!("  --color-base16: {};\n", color));
        }
        if let Some(ref color) = self.palette.base17 {
            css.push_str(&format!("  --color-base17: {};\n", color));
        }

        css.push_str("\n  /* Semantic Color Mappings */\n");

        // Generate semantic variables by resolving base color references
        css.push_str(&format!(
            "  --color-background: var(--color-{});\n",
            self.semantic.background
        ));
        css.push_str(&format!(
            "  --color-foreground: var(--color-{});\n",
            self.semantic.foreground
        ));
        css.push_str(&format!(
            "  --color-primary: var(--color-{});\n",
            self.semantic.primary
        ));
        css.push_str(&format!(
            "  --color-secondary: var(--color-{});\n",
            self.semantic.secondary
        ));
        css.push_str(&format!(
            "  --color-accent: var(--color-{});\n",
            self.semantic.accent
        ));
        css.push_str(&format!(
            "  --color-muted: var(--color-{});\n",
            self.semantic.muted
        ));
        css.push_str(&format!(
            "  --color-border: var(--color-{});\n",
            self.semantic.border
        ));
        css.push_str(&format!(
            "  --color-success: var(--color-{});\n",
            self.semantic.success
        ));
        css.push_str(&format!(
            "  --color-warning: var(--color-{});\n",
            self.semantic.warning
        ));
        css.push_str(&format!(
            "  --color-error: var(--color-{});\n",
            self.semantic.error
        ));
        css.push_str(&format!(
            "  --color-info: var(--color-{});\n",
            self.semantic.info
        ));

        css.push_str("\n  /* Legacy Compatibility */\n");
        // Map to existing CSS variables for backwards compatibility
        css.push_str(&format!(
            "  --background: var(--color-{});\n",
            self.semantic.background
        ));
        css.push_str(&format!(
            "  --foreground: var(--color-{});\n",
            self.semantic.foreground
        ));
        css.push_str(&format!(
            "  --primary: var(--color-{});\n",
            self.semantic.primary
        ));
        css.push_str(&format!(
            "  --accent: var(--color-{});\n",
            self.semantic.accent
        ));
        css.push_str(&format!(
            "  --muted: var(--color-{});\n",
            self.semantic.muted
        ));
        css.push_str(&format!(
            "  --border: var(--color-{});\n",
            self.semantic.border
        ));

        // Additional mappings for existing variables
        css.push_str(&format!(
            "  --card: var(--color-{});\n",
            self.semantic.muted
        ));
        css.push_str(&format!(
            "  --card-foreground: var(--color-{});\n",
            self.semantic.foreground
        ));
        css.push_str(&format!(
            "  --popover: var(--color-{});\n",
            self.semantic.background
        ));
        css.push_str(&format!(
            "  --popover-foreground: var(--color-{});\n",
            self.semantic.foreground
        ));
        css.push_str(&format!(
            "  --primary-foreground: var(--color-{});\n",
            self.semantic.background
        ));
        css.push_str(&format!(
            "  --secondary: var(--color-{});\n",
            self.semantic.secondary
        ));
        css.push_str(&format!(
            "  --secondary-foreground: var(--color-{});\n",
            self.semantic.foreground
        ));
        css.push_str(&format!(
            "  --muted-foreground: var(--color-{});\n",
            self.semantic.muted
        ));
        css.push_str(&format!(
            "  --accent-foreground: var(--color-{});\n",
            self.semantic.background
        ));
        css.push_str(&format!(
            "  --destructive: var(--color-{});\n",
            self.semantic.error
        ));
        css.push_str(&format!(
            "  --input: var(--color-{});\n",
            self.semantic.border
        ));
        css.push_str(&format!(
            "  --ring: var(--color-{});\n",
            self.semantic.primary
        ));

        css
    }

    /// Generate a complete CSS theme block
    pub fn generate_css_theme(&self) -> String {
        format!(":root {{\n{}}}\n", self.generate_css_variables())
    }

    /// Resolve a semantic color to its actual hex value
    pub fn resolve_semantic_color(&self, semantic_name: &str) -> Option<String> {
        let base_color = match semantic_name {
            "background" => &self.semantic.background,
            "foreground" => &self.semantic.foreground,
            "primary" => &self.semantic.primary,
            "secondary" => &self.semantic.secondary,
            "accent" => &self.semantic.accent,
            "muted" => &self.semantic.muted,
            "border" => &self.semantic.border,
            "success" => &self.semantic.success,
            "warning" => &self.semantic.warning,
            "error" => &self.semantic.error,
            "info" => &self.semantic.info,
            _ => return None,
        };

        self.resolve_base_color(base_color)
    }

    /// Resolve a base color reference to its actual hex value
    pub fn resolve_base_color(&self, base_color: &str) -> Option<String> {
        match base_color {
            "base00" => self.palette.base00.clone(),
            "base01" => self.palette.base01.clone(),
            "base02" => self.palette.base02.clone(),
            "base03" => self.palette.base03.clone(),
            "base04" => self.palette.base04.clone(),
            "base05" => self.palette.base05.clone(),
            "base06" => self.palette.base06.clone(),
            "base07" => self.palette.base07.clone(),
            "base08" => self.palette.base08.clone(),
            "base09" => self.palette.base09.clone(),
            "base0A" => self.palette.base0a.clone(),
            "base0B" => self.palette.base0b.clone(),
            "base0C" => self.palette.base0c.clone(),
            "base0D" => self.palette.base0d.clone(),
            "base0E" => self.palette.base0e.clone(),
            "base0F" => self.palette.base0f.clone(),
            "base10" => self.palette.base10.clone(),
            "base11" => self.palette.base11.clone(),
            "base12" => self.palette.base12.clone(),
            "base13" => self.palette.base13.clone(),
            "base14" => self.palette.base14.clone(),
            "base15" => self.palette.base15.clone(),
            "base16" => self.palette.base16.clone(),
            "base17" => self.palette.base17.clone(),
            _ => None,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            system: ThemeSystem::Base16,
            scheme: "base16-default-dark".to_string(),
            palette: ThemePalette::default(),
            semantic: SemanticColors::default(),
            inherits: None,
            variants: None,
            overrides: None,
            name: Some("Default Dark".to_string()),
            author: None,
            description: Some("Default dark theme".to_string()),
            variant: Some(ThemeVariant::Dark),
        }
    }
}

/// Theme loader for managing theme files and inheritance
#[derive(Debug)]
pub struct ThemeLoader {
    theme_dirs: Vec<PathBuf>,
    built_in_themes: BTreeMap<String, Theme>,
}

impl ThemeLoader {
    pub fn new() -> Self {
        Self::with_config(None)
    }

    pub fn with_config(themes_dir: Option<PathBuf>) -> Self {
        let mut loader = Self {
            theme_dirs: Vec::new(),
            built_in_themes: BTreeMap::new(),
        };

        // Add configured themes directory or default
        if let Some(themes_dir) = themes_dir {
            loader.theme_dirs.push(themes_dir);
        } else {
            // Default to ~/.config/themes (tinty compatible)
            if let Some(home_dir) = dirs::home_dir() {
                loader
                    .theme_dirs
                    .push(home_dir.join(".config").join("themes"));
            }
            // Also check system config directory for backwards compatibility
            if let Some(config_dir) = dirs::config_dir() {
                loader
                    .theme_dirs
                    .push(config_dir.join("lst").join("themes"));
            }
        }

        // Load built-in themes
        loader.load_built_in_themes();

        loader
    }

    /// Add a theme directory to search path
    pub fn add_theme_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.theme_dirs.push(path.as_ref().to_path_buf());
    }

    /// Load a theme by name
    pub fn load_theme(&self, name: &str) -> Result<Theme> {
        // Check built-in themes first
        if let Some(theme) = self.built_in_themes.get(name) {
            return Ok(theme.clone());
        }

        // Search theme directories
        for theme_dir in &self.theme_dirs {
            // Try legacy .toml format first
            let toml_path = theme_dir.join(format!("{}.toml", name));
            if toml_path.exists() {
                return self.load_theme_from_file(&toml_path);
            }

            // Try tinty-compatible format: base16-theme-name or base24-theme-name
            if let Some((system, theme_name)) = self.parse_theme_name(name) {
                let yaml_path = theme_dir.join(&system).join(format!("{}.yaml", theme_name));
                if yaml_path.exists() {
                    return self.load_theme_from_yaml_file(&yaml_path, name);
                }

                let yml_path = theme_dir.join(&system).join(format!("{}.yml", theme_name));
                if yml_path.exists() {
                    return self.load_theme_from_yaml_file(&yml_path, name);
                }
            }
        }

        anyhow::bail!("Theme '{}' not found", name);
    }

    /// Parse theme name to extract system (base16/base24) and theme name
    fn parse_theme_name(&self, name: &str) -> Option<(String, String)> {
        if let Some(rest) = name.strip_prefix("base16-") {
            Some(("base16".to_string(), rest.to_string()))
        } else if let Some(rest) = name.strip_prefix("base24-") {
            Some(("base24".to_string(), rest.to_string()))
        } else {
            None
        }
    }

    /// Load theme from a specific file
    pub fn load_theme_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Theme> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read theme file: {}", path.as_ref().display()))?;

        let mut theme: Theme = toml::from_str(&content)
            .with_context(|| format!("Failed to parse theme file: {}", path.as_ref().display()))?;

        // Apply inheritance if specified
        if let Some(ref parent_name) = theme.inherits.clone() {
            let parent_theme = self
                .load_theme(parent_name)
                .with_context(|| format!("Failed to load parent theme: {}", parent_name))?;
            theme = self.merge_themes(parent_theme, theme)?;
        }

        // Apply overrides
        if let Some(ref overrides) = theme.overrides.clone() {
            theme = self.apply_overrides(theme, overrides)?;
        }

        // Validate theme
        self.validate_theme(&theme)?;

        Ok(theme)
    }

    /// Load theme from a YAML file (tinty format)
    pub fn load_theme_from_yaml_file<P: AsRef<Path>>(
        &self,
        path: P,
        theme_name: &str,
    ) -> Result<Theme> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read theme file: {}", path.as_ref().display()))?;

        let mut theme: Theme = serde_yaml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse YAML theme file: {}",
                path.as_ref().display()
            )
        })?;

        // Set the scheme name to match the requested theme name
        theme.scheme = theme_name.to_string();

        // Apply inheritance if specified
        if let Some(ref parent_name) = theme.inherits.clone() {
            let parent_theme = self
                .load_theme(parent_name)
                .with_context(|| format!("Failed to load parent theme: {}", parent_name))?;
            theme = self.merge_themes(parent_theme, theme)?;
        }

        // Apply overrides
        if let Some(ref overrides) = theme.overrides.clone() {
            theme = self.apply_overrides(theme, overrides)?;
        }

        // Validate theme
        self.validate_theme(&theme)?;

        Ok(theme)
    }

    /// List all available themes
    pub fn list_themes(&self) -> Vec<String> {
        let mut themes = Vec::new();

        // Add built-in themes
        themes.extend(self.built_in_themes.keys().cloned());

        // Add themes from directories
        for theme_dir in &self.theme_dirs {
            // Check for direct theme files (legacy .toml format)
            if let Ok(entries) = std::fs::read_dir(theme_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".toml") {
                            let theme_name = name.trim_end_matches(".toml");
                            if !themes.contains(&theme_name.to_string()) {
                                themes.push(theme_name.to_string());
                            }
                        }
                    }
                }
            }

            // Check for base16 and base24 subdirectories (tinty format)
            for system in &["base16", "base24"] {
                let system_dir = theme_dir.join(system);
                if let Ok(entries) = std::fs::read_dir(&system_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".yaml") || name.ends_with(".yml") {
                                let base_name =
                                    name.trim_end_matches(".yaml").trim_end_matches(".yml");
                                let theme_name = format!("{}-{}", system, base_name);
                                if !themes.contains(&theme_name) {
                                    themes.push(theme_name);
                                }
                            }
                        }
                    }
                }
            }
        }

        themes.sort();
        themes
    }

    /// Get theme information
    pub fn get_theme_info(&self, name: &str) -> Result<ThemeInfo> {
        let theme = self.load_theme(name)?;
        Ok(ThemeInfo {
            name: theme.name.unwrap_or_else(|| name.to_string()),
            scheme: theme.scheme,
            author: theme.author,
            description: theme.description,
            system: theme.system,
            variant: theme.variant,
        })
    }

    /// Validate a theme
    pub fn validate_theme(&self, theme: &Theme) -> Result<()> {
        // Check that required base16 colors are present
        let required_base16 = [
            ("base00", &theme.palette.base00),
            ("base01", &theme.palette.base01),
            ("base02", &theme.palette.base02),
            ("base03", &theme.palette.base03),
            ("base04", &theme.palette.base04),
            ("base05", &theme.palette.base05),
            ("base06", &theme.palette.base06),
            ("base07", &theme.palette.base07),
            ("base08", &theme.palette.base08),
            ("base09", &theme.palette.base09),
            ("base0A", &theme.palette.base0a),
            ("base0B", &theme.palette.base0b),
            ("base0C", &theme.palette.base0c),
            ("base0D", &theme.palette.base0d),
            ("base0E", &theme.palette.base0e),
            ("base0F", &theme.palette.base0f),
        ];

        for (name, color) in required_base16 {
            if color.is_none() {
                anyhow::bail!("Missing required color: {}", name);
            }

            // Validate color format
            if let Some(color_value) = color {
                if !is_valid_color(color_value) {
                    anyhow::bail!("Invalid color format for {}: {}", name, color_value);
                }
            }
        }

        Ok(())
    }

    /// Merge parent and child themes
    fn merge_themes(&self, mut parent: Theme, child: Theme) -> Result<Theme> {
        // Merge palette (child overrides parent)
        if let Some(base00) = child.palette.base00 {
            parent.palette.base00 = Some(base00);
        }
        if let Some(base01) = child.palette.base01 {
            parent.palette.base01 = Some(base01);
        }
        if let Some(base02) = child.palette.base02 {
            parent.palette.base02 = Some(base02);
        }
        if let Some(base03) = child.palette.base03 {
            parent.palette.base03 = Some(base03);
        }
        if let Some(base04) = child.palette.base04 {
            parent.palette.base04 = Some(base04);
        }
        if let Some(base05) = child.palette.base05 {
            parent.palette.base05 = Some(base05);
        }
        if let Some(base06) = child.palette.base06 {
            parent.palette.base06 = Some(base06);
        }
        if let Some(base07) = child.palette.base07 {
            parent.palette.base07 = Some(base07);
        }
        if let Some(base08) = child.palette.base08 {
            parent.palette.base08 = Some(base08);
        }
        if let Some(base09) = child.palette.base09 {
            parent.palette.base09 = Some(base09);
        }
        if let Some(base0a) = child.palette.base0a {
            parent.palette.base0a = Some(base0a);
        }
        if let Some(base0b) = child.palette.base0b {
            parent.palette.base0b = Some(base0b);
        }
        if let Some(base0c) = child.palette.base0c {
            parent.palette.base0c = Some(base0c);
        }
        if let Some(base0d) = child.palette.base0d {
            parent.palette.base0d = Some(base0d);
        }
        if let Some(base0e) = child.palette.base0e {
            parent.palette.base0e = Some(base0e);
        }
        if let Some(base0f) = child.palette.base0f {
            parent.palette.base0f = Some(base0f);
        }

        // Base24 colors
        if let Some(base10) = child.palette.base10 {
            parent.palette.base10 = Some(base10);
        }
        if let Some(base11) = child.palette.base11 {
            parent.palette.base11 = Some(base11);
        }
        if let Some(base12) = child.palette.base12 {
            parent.palette.base12 = Some(base12);
        }
        if let Some(base13) = child.palette.base13 {
            parent.palette.base13 = Some(base13);
        }
        if let Some(base14) = child.palette.base14 {
            parent.palette.base14 = Some(base14);
        }
        if let Some(base15) = child.palette.base15 {
            parent.palette.base15 = Some(base15);
        }
        if let Some(base16) = child.palette.base16 {
            parent.palette.base16 = Some(base16);
        }
        if let Some(base17) = child.palette.base17 {
            parent.palette.base17 = Some(base17);
        }

        // Override other fields from child
        parent.scheme = child.scheme;
        parent.system = child.system;
        if child.name.is_some() {
            parent.name = child.name;
        }
        if child.author.is_some() {
            parent.author = child.author;
        }
        if child.description.is_some() {
            parent.description = child.description;
        }
        if child.variant.is_some() {
            parent.variant = child.variant;
        }
        if child.variants.is_some() {
            parent.variants = child.variants;
        }
        if child.overrides.is_some() {
            parent.overrides = child.overrides;
        }

        Ok(parent)
    }

    /// Apply theme overrides
    fn apply_overrides(&self, mut theme: Theme, overrides: &ThemeOverrides) -> Result<Theme> {
        // Apply palette overrides
        for (key, value) in &overrides.palette {
            match key.as_str() {
                "base00" => theme.palette.base00 = Some(value.clone()),
                "base01" => theme.palette.base01 = Some(value.clone()),
                "base02" => theme.palette.base02 = Some(value.clone()),
                "base03" => theme.palette.base03 = Some(value.clone()),
                "base04" => theme.palette.base04 = Some(value.clone()),
                "base05" => theme.palette.base05 = Some(value.clone()),
                "base06" => theme.palette.base06 = Some(value.clone()),
                "base07" => theme.palette.base07 = Some(value.clone()),
                "base08" => theme.palette.base08 = Some(value.clone()),
                "base09" => theme.palette.base09 = Some(value.clone()),
                "base0A" => theme.palette.base0a = Some(value.clone()),
                "base0B" => theme.palette.base0b = Some(value.clone()),
                "base0C" => theme.palette.base0c = Some(value.clone()),
                "base0D" => theme.palette.base0d = Some(value.clone()),
                "base0E" => theme.palette.base0e = Some(value.clone()),
                "base0F" => theme.palette.base0f = Some(value.clone()),
                _ => {} // Ignore unknown palette keys
            }
        }

        // Apply semantic overrides
        if let Some(ref semantic_overrides) = overrides.semantic {
            for (key, value) in semantic_overrides {
                match key.as_str() {
                    "background" => theme.semantic.background = value.clone(),
                    "foreground" => theme.semantic.foreground = value.clone(),
                    "primary" => theme.semantic.primary = value.clone(),
                    "secondary" => theme.semantic.secondary = value.clone(),
                    "accent" => theme.semantic.accent = value.clone(),
                    "muted" => theme.semantic.muted = value.clone(),
                    "border" => theme.semantic.border = value.clone(),
                    "success" => theme.semantic.success = value.clone(),
                    "warning" => theme.semantic.warning = value.clone(),
                    "error" => theme.semantic.error = value.clone(),
                    "info" => theme.semantic.info = value.clone(),
                    _ => {} // Ignore unknown semantic keys
                }
            }
        }

        Ok(theme)
    }

    /// Load built-in themes
    fn load_built_in_themes(&mut self) {
        // Base16 Default Dark
        self.built_in_themes.insert(
            "base16-default-dark".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-default-dark".to_string(),
                palette: ThemePalette {
                    base00: Some("#181818".to_string()), // Background
                    base01: Some("#282828".to_string()), // Lighter Background
                    base02: Some("#383838".to_string()), // Selection Background
                    base03: Some("#585858".to_string()), // Comments
                    base04: Some("#b8b8b8".to_string()), // Dark Foreground
                    base05: Some("#d8d8d8".to_string()), // Default Foreground
                    base06: Some("#e8e8e8".to_string()), // Light Foreground
                    base07: Some("#f8f8f8".to_string()), // Light Background
                    base08: Some("#ab4642".to_string()), // Red
                    base09: Some("#dc9656".to_string()), // Orange
                    base0a: Some("#f7ca88".to_string()), // Yellow
                    base0b: Some("#a1b56c".to_string()), // Green
                    base0c: Some("#86c1b9".to_string()), // Cyan
                    base0d: Some("#7cafc2".to_string()), // Blue
                    base0e: Some("#ba8baf".to_string()), // Purple
                    base0f: Some("#a16946".to_string()), // Brown
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Base16 Default Dark".to_string()),
                author: Some("Chris Kempson".to_string()),
                description: Some("Base16 default dark theme".to_string()),
                variant: Some(ThemeVariant::Dark),
            },
        );

        // Base16 Default Light
        self.built_in_themes.insert(
            "base16-default-light".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-default-light".to_string(),
                palette: ThemePalette {
                    base00: Some("#f8f8f8".to_string()), // Background
                    base01: Some("#e8e8e8".to_string()), // Lighter Background
                    base02: Some("#d8d8d8".to_string()), // Selection Background
                    base03: Some("#b8b8b8".to_string()), // Comments
                    base04: Some("#585858".to_string()), // Dark Foreground
                    base05: Some("#383838".to_string()), // Default Foreground
                    base06: Some("#282828".to_string()), // Light Foreground
                    base07: Some("#181818".to_string()), // Light Background
                    base08: Some("#ab4642".to_string()), // Red
                    base09: Some("#dc9656".to_string()), // Orange
                    base0a: Some("#f7ca88".to_string()), // Yellow
                    base0b: Some("#a1b56c".to_string()), // Green
                    base0c: Some("#86c1b9".to_string()), // Cyan
                    base0d: Some("#7cafc2".to_string()), // Blue
                    base0e: Some("#ba8baf".to_string()), // Purple
                    base0f: Some("#a16946".to_string()), // Brown
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Base16 Default Light".to_string()),
                author: Some("Chris Kempson".to_string()),
                description: Some("Base16 default light theme".to_string()),
                variant: Some(ThemeVariant::Light),
            },
        );

        // Catppuccin Mocha (popular dark theme)
        self.built_in_themes.insert(
            "base16-catppuccin-mocha".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-catppuccin-mocha".to_string(),
                palette: ThemePalette {
                    base00: Some("#1e1e2e".to_string()), // base
                    base01: Some("#181825".to_string()), // mantle
                    base02: Some("#313244".to_string()), // surface0
                    base03: Some("#45475a".to_string()), // surface1
                    base04: Some("#585b70".to_string()), // surface2
                    base05: Some("#cdd6f4".to_string()), // text
                    base06: Some("#f5e0dc".to_string()), // rosewater
                    base07: Some("#b4befe".to_string()), // lavender
                    base08: Some("#f38ba8".to_string()), // red
                    base09: Some("#fab387".to_string()), // peach
                    base0a: Some("#f9e2af".to_string()), // yellow
                    base0b: Some("#a6e3a1".to_string()), // green
                    base0c: Some("#94e2d5".to_string()), // teal
                    base0d: Some("#89b4fa".to_string()), // blue
                    base0e: Some("#cba6f7".to_string()), // mauve
                    base0f: Some("#f2cdcd".to_string()), // flamingo
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Catppuccin Mocha".to_string()),
                author: Some("https://github.com/catppuccin/catppuccin".to_string()),
                description: Some("Soothing pastel theme for the high-spirited!".to_string()),
                variant: Some(ThemeVariant::Dark),
            },
        );

        // Nord (arctic, north-bluish color palette)
        self.built_in_themes.insert(
            "base16-nord".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-nord".to_string(),
                palette: ThemePalette {
                    base00: Some("#2E3440".to_string()),
                    base01: Some("#3B4252".to_string()),
                    base02: Some("#434C5E".to_string()),
                    base03: Some("#4C566A".to_string()),
                    base04: Some("#D8DEE9".to_string()),
                    base05: Some("#E5E9F0".to_string()),
                    base06: Some("#ECEFF4".to_string()),
                    base07: Some("#8FBCBB".to_string()),
                    base08: Some("#BF616A".to_string()),
                    base09: Some("#D08770".to_string()),
                    base0a: Some("#EBCB8B".to_string()),
                    base0b: Some("#A3BE8C".to_string()),
                    base0c: Some("#88C0D0".to_string()),
                    base0d: Some("#81A1C1".to_string()),
                    base0e: Some("#B48EAD".to_string()),
                    base0f: Some("#5E81AC".to_string()),
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Nord".to_string()),
                author: Some("arcticicestudio".to_string()),
                description: Some("An arctic, north-bluish color palette".to_string()),
                variant: Some(ThemeVariant::Dark),
            },
        );

        // Tokyo Night Dark
        self.built_in_themes.insert(
            "base16-tokyo-night-dark".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-tokyo-night-dark".to_string(),
                palette: ThemePalette {
                    base00: Some("#1A1B26".to_string()),
                    base01: Some("#16161E".to_string()),
                    base02: Some("#2F3549".to_string()),
                    base03: Some("#444B6A".to_string()),
                    base04: Some("#787C99".to_string()),
                    base05: Some("#A9B1D6".to_string()),
                    base06: Some("#CBCCD1".to_string()),
                    base07: Some("#D5D6DB".to_string()),
                    base08: Some("#C0CAF5".to_string()),
                    base09: Some("#A9B1D6".to_string()),
                    base0a: Some("#0DB9D7".to_string()),
                    base0b: Some("#9ECE6A".to_string()),
                    base0c: Some("#B4F9F8".to_string()),
                    base0d: Some("#2AC3DE".to_string()),
                    base0e: Some("#BB9AF7".to_string()),
                    base0f: Some("#F7768E".to_string()),
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Tokyo Night Dark".to_string()),
                author: Some("Michaël Ball".to_string()),
                description: Some(
                    "A clean, dark theme celebrating the lights of Downtown Tokyo at night"
                        .to_string(),
                ),
                variant: Some(ThemeVariant::Dark),
            },
        );

        // Everforest
        self.built_in_themes.insert(
            "base16-everforest".to_string(),
            Theme {
                system: ThemeSystem::Base16,
                scheme: "base16-everforest".to_string(),
                palette: ThemePalette {
                    base00: Some("#2d353b".to_string()), // bg0
                    base01: Some("#343f44".to_string()), // bg1
                    base02: Some("#475258".to_string()), // bg3
                    base03: Some("#859289".to_string()), // grey1
                    base04: Some("#9da9a0".to_string()), // grey2
                    base05: Some("#d3c6aa".to_string()), // fg
                    base06: Some("#e6e2cc".to_string()), // bg3 light
                    base07: Some("#fdf6e3".to_string()), // bg0 light
                    base08: Some("#e67e80".to_string()), // red
                    base09: Some("#e69875".to_string()), // orange
                    base0a: Some("#dbbc7f".to_string()), // yellow
                    base0b: Some("#a7c080".to_string()), // green
                    base0c: Some("#83c092".to_string()), // aqua
                    base0d: Some("#7fbbb3".to_string()), // blue
                    base0e: Some("#d699b6".to_string()), // purple
                    base0f: Some("#9da9a0".to_string()), // grey2
                    ..Default::default()
                },
                semantic: SemanticColors::default(),
                inherits: None,
                variants: None,
                overrides: None,
                name: Some("Everforest".to_string()),
                author: Some("Sainnhe Park (https://github.com/sainnhe)".to_string()),
                description: Some("Comfortable & Pleasant Color Scheme".to_string()),
                variant: Some(ThemeVariant::Dark),
            },
        );

        // Backward compatibility aliases (without base16- prefix)
        if let Some(catppuccin_theme) = self.built_in_themes.get("base16-catppuccin-mocha").cloned()
        {
            self.built_in_themes
                .insert("catppuccin-mocha".to_string(), catppuccin_theme);
        }
    }
}

impl Default for ThemeLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ThemeInfo {
    pub name: String,
    pub scheme: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub system: ThemeSystem,
    pub variant: Option<ThemeVariant>,
}

/// Validate color format (hex colors)
fn is_valid_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }

    let hex = &color[1..];
    if hex.len() != 6 && hex.len() != 3 {
        return false;
    }

    hex.chars().all(|c| c.is_ascii_hexdigit())
}

// Default semantic color mappings
fn default_background() -> String {
    "base00".to_string()
}
fn default_foreground() -> String {
    "base05".to_string()
}
fn default_primary() -> String {
    "base0D".to_string()
}
fn default_secondary() -> String {
    "base06".to_string()
}
fn default_accent() -> String {
    "base0E".to_string()
}
fn default_muted() -> String {
    "base03".to_string()
}
fn default_border() -> String {
    "base02".to_string()
}
fn default_success() -> String {
    "base0B".to_string()
}
fn default_warning() -> String {
    "base0A".to_string()
}
fn default_error() -> String {
    "base08".to_string()
}
fn default_info() -> String {
    "base0C".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_validation() {
        assert!(is_valid_color("#ff0000"));
        assert!(is_valid_color("#FF0000"));
        assert!(is_valid_color("#f00"));
        assert!(!is_valid_color("ff0000"));
        assert!(!is_valid_color("#gg0000"));
        assert!(!is_valid_color("#ff00"));
    }

    #[test]
    fn test_theme_loader() {
        let loader = ThemeLoader::new();
        let themes = loader.list_themes();
        assert!(themes.contains(&"base16-default-dark".to_string()));
        assert!(themes.contains(&"base16-default-light".to_string()));
        assert!(themes.contains(&"base16-catppuccin-mocha".to_string()));
        assert!(themes.contains(&"base16-nord".to_string()));
        assert!(themes.contains(&"base16-tokyo-night-dark".to_string()));
        assert!(themes.contains(&"base16-everforest".to_string()));
    }

    #[test]
    fn test_load_built_in_theme() {
        let loader = ThemeLoader::new();
        let theme = loader.load_theme("base16-catppuccin-mocha").unwrap();
        assert_eq!(theme.scheme, "base16-catppuccin-mocha");
        assert_eq!(theme.system, ThemeSystem::Base16);
        assert!(theme.palette.base00.is_some());
    }

    #[test]
    fn test_css_generation() {
        let loader = ThemeLoader::new();
        let theme = loader.load_theme("base16-catppuccin-mocha").unwrap();
        let css = theme.generate_css_variables();

        // Check that base colors are generated
        assert!(css.contains("--color-base00: #1e1e2e"));
        assert!(css.contains("--color-base05: #cdd6f4"));

        // Check that semantic mappings are generated
        assert!(css.contains("--color-background: var(--color-base00)"));
        assert!(css.contains("--color-foreground: var(--color-base05)"));

        // Check legacy compatibility
        assert!(css.contains("--background: var(--color-base00)"));
        assert!(css.contains("--foreground: var(--color-base05)"));
    }

    #[test]
    fn test_color_resolution() {
        let loader = ThemeLoader::new();
        let theme = loader.load_theme("base16-catppuccin-mocha").unwrap();

        // Test base color resolution
        assert_eq!(
            theme.resolve_base_color("base00"),
            Some("#1e1e2e".to_string())
        );
        assert_eq!(
            theme.resolve_base_color("base05"),
            Some("#cdd6f4".to_string())
        );

        // Test semantic color resolution
        assert_eq!(
            theme.resolve_semantic_color("background"),
            Some("#1e1e2e".to_string())
        );
        assert_eq!(
            theme.resolve_semantic_color("foreground"),
            Some("#cdd6f4".to_string())
        );
    }
}
