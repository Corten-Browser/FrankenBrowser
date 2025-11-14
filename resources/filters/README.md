# Ad Blocker Filters

## EasyList

The `easylist.txt` file should be downloaded from:
https://easylist.to/easylist/easylist.txt

### Download Script

```bash
curl -o easylist.txt https://easylist.to/easylist/easylist.txt
```

### Build Integration

The `build.rs` script will automatically download EasyList if not present during build.

### Manual Download

If needed during development:

```bash
cd resources/filters
curl -o easylist.txt https://easylist.to/easylist/easylist.txt
```

## Filter Format

EasyList uses Adblock Plus filter syntax. The `adblock` crate handles parsing and matching.

## Custom Filters

Custom filters can be added in the `custom_filters` array in `resources/config/default_settings.toml`.
