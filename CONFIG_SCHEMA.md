# LST Configuration Schema

LST now provides JSON schema validation for configuration files to help catch errors and provide autocomplete in supported editors.

## Schema Reference in Config Files

LST automatically includes schema references in **all** generated configuration files:

```toml
# LST Configuration File
# Schema: https://json-schema.org/draft-07/schema#
# LST Configuration Schema: ./lst-config-schema.json
# For LSP/editor validation, configure your editor to use the schema above

[fuzzy]
threshold = 50.0
max_suggestions = 7
```

**When LST creates a config file automatically:**
- ✅ Schema reference is included at the top
- ✅ Works for `~/.config/lst/lst.toml` (first run)
- ✅ Works for sync daemon configs
- ✅ No manual editing required

**For existing config files:**
- Add the schema reference comments manually, or
- Regenerate with `lst schema` and copy the header

### Schema Reference Formats

Different LSP servers support various ways to reference schemas in TOML files:

**Comment-based (LST default):**
```toml
# LST Configuration Schema: ./lst-config-schema.json
```

**JSON Schema style:**
```toml
# $schema = "./lst-config-schema.json"
```

**Taplo-specific:**
```toml
#:schema ./lst-config-schema.json
```

**Inline schema reference:**
```toml
schema = "./lst-config-schema.json"  # Some LSPs read this key
```

## Generated Schema

The `lst-config-schema.json` file contains the complete JSON schema for LST's TOML configuration, automatically generated from the Rust structs using [schemars](https://github.com/GREsau/schemars).

## Editor Setup

### Neovim with LSP

#### Option 1: Using yaml-language-server (recommended)

1. Install `yaml-language-server`:
   ```bash
   npm install -g yaml-language-server
   ```

2. Configure Neovim to associate TOML files with YAML LSP and use the schema:

   ```lua
   -- In your init.lua or plugin config
   require('lspconfig').yamlls.setup {
     settings = {
       yaml = {
         schemas = {
           ["./lst-config-schema.json"] = "*.toml"
         }
       }
     }
   }
   ```

#### Option 2: Using taplo (TOML LSP)

1. Install `taplo`:
   ```bash
   cargo install taplo-cli --features lsp
   ```

2. Configure Neovim:

   ```lua
   require('lspconfig').taplo.setup {
     settings = {
       evenBetterToml = {
         schema = {
           associations = {
             ["./lst-config-schema.json"] = "*.toml"
           }
         }
       }
     }
   }
   ```

   **Or let taplo auto-detect the schema from the file:**
   ```lua
   require('lspconfig').taplo.setup {
     settings = {
       evenBetterToml = {
         schema = {
           enabled = true  -- taplo will look for schema references in TOML files
         }
       }
     }
   }
   ```

### VS Code

1. Install the "Even Better TOML" extension
2. Add to your `settings.json`:

   ```json
   {
     "evenBetterToml.schema.associations": {
       "*.toml": "./lst-config-schema.json"
     }
   }
   ```

### Other Editors

Most LSP-compatible editors that support TOML can use this schema by:

1. Configuring the TOML LSP to use `./lst-config-schema.json`
2. Setting up file associations for `*.toml` files

## Schema Features

The generated schema includes:

- ✅ **Type validation** for all configuration fields
- ✅ **Default values** clearly documented
- ✅ **Required vs optional** field distinction
- ✅ **Enum validation** for predefined options
- ✅ **Range validation** for numeric fields
- ✅ **Pattern validation** for string fields

## Example Configuration with Validation

```toml
# This will show validation errors if:
# - threshold is not a number
# - max_suggestions is not an integer
# - invalid section names are used

[fuzzy]
threshold = 50.0      # ✅ Valid: number between 0-200
max_suggestions = 7   # ✅ Valid: positive integer

[paths]
content_dir = "~/lst"  # ✅ Valid: string path

[server]
host = "127.0.0.1"     # ✅ Valid: string
port = 5673           # ✅ Valid: positive integer
```

## Regenerating the Schema

If you modify the LST configuration structure, regenerate the schema:

```bash
# From the project root
cargo run --bin lst -- schema > lst-config-schema.json
```

## Schema Coverage

The schema covers all LST configuration sections:

- **fuzzy**: Fuzzy matching settings
- **ui**: User interface preferences
- **paths**: File system paths
- **server**: Server daemon configuration
- **sync**: Synchronization settings
- **storage**: Storage backend settings
- **email**: Email configuration (server only)

Note: Some complex types like `Theme` are excluded from the schema for simplicity.