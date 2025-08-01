# Include Folder Macro

## What's this

```rust
// imagine the struct of current folder is 
// ./
// ./lib.rs
// ./example.rs
// ./example/code.rs
// ./example/code2.rs
include_folder!(".");

// This will expand to
pub mod example {
    pub mod code;
    pub mod code2;
}
```

## Usage

- Run `cargo add include-folder-macro`
- Add `include_folder!()` macro to your lib.rs or main.rs
