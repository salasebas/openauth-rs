# openauth-i18n

Internationalization plugin for OpenAuth-RS.

## Status

This package is in experimental beta. Locale detection, translation keys, and
plugin behavior may change before stable release.

## What It Provides

`openauth-i18n` adds localized auth responses through translation dictionaries
and locale detection strategies. It supports header-based locale detection by
default and can be configured for cookies, callbacks, or session-derived locale
values.

## Example

```rust
use indexmap::IndexMap;
use openauth::OpenAuth;
use openauth_i18n::{i18n, translation_dictionary, I18nOptions};

let mut translations = IndexMap::new();
translations.insert(
    "en".to_owned(),
    translation_dictionary([("invalid_email", "Invalid email")]),
);

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .plugin(i18n(I18nOptions::new(translations))?)
    .build()?;
```

Keep application-specific copy in dictionaries and leave authentication logic in
the core and plugin crates.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
