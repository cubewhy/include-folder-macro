# Include Folder Macro

## What's this

```rust
// imagine the struct of current folder is 
// ./src
// ./src/lib.rs
// ./src/example.rs
// ./src/example/code.rs
// ./src/example/code2.rs
include_all_modules!("src");

// This will expand to
pub mod example {
    pub mod code;
    pub mod code2;
}
```

## Usage

- Run `cargo add include-folder-macro`
- Add `include_folder!()` or `include_all_modules!("src")` macro to your lib.rs or main.rs
