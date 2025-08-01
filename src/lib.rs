use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn include_all_modules(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_path = PathBuf::from(manifest_dir).join(&path_str);

    if !root_path.is_dir() {
        let error_msg = format!("Path '{}' is not an valid module", root_path.display());
        return syn::Error::new_spanned(path_lit, error_msg)
            .to_compile_error()
            .into();
    }

    // ignore lib.rs, main.rs, 和 mod.rs
    let ignore_list = &["lib.rs", "main.rs", "mod.rs"];

    match generate_modules_recursive(&root_path, ignore_list) {
        Ok(tokens) => tokens.into(),
        Err(e) => e,
    }
}

#[proc_macro]
pub fn include_folder(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut root_path = PathBuf::from(manifest_dir);
    root_path.push(&path_str);

    let module_name = Path::new(&path_str)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(".", "_"))
        .expect("Path should contains an valid module");

    if !root_path.is_dir() {
        let error_msg = format!("Path '{}' is not an valid dir", root_path.display());
        return syn::Error::new_spanned(path_lit, error_msg)
            .to_compile_error()
            .into();
    }

    let modules = match generate_modules_recursive(&root_path, &["mod.rs"]) {
        Ok(tokens) => tokens,
        Err(e) => return e.into(),
    };

    let top_module_ident = Ident::new(&module_name, Span::call_site());
    let expanded = quote! {
        pub mod #top_module_ident {
            #modules
        }
    };

    expanded.into()
}

fn generate_modules_recursive(
    dir: &Path,
    files_to_ignore: &[&str],
) -> Result<proc_macro2::TokenStream, TokenStream> {
    let mut modules = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(to_compile_error(format!(
                "Cannot read dir '{}': {}",
                dir.display(),
                e
            )));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|e| to_compile_error(e.to_string()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if files_to_ignore.contains(&file_name_str.as_ref()) {
            continue;
        }

        if path.is_dir() {
            let inner_mods = generate_modules_recursive(&path, &["mod.rs"])?;

            if !inner_mods.is_empty() {
                let mod_name = Ident::new(&file_name_str, Span::call_site());
                modules.push(quote! {
                    pub mod #mod_name {
                        #inner_mods
                    }
                });
            }
        } else if path.is_file() {
            if path.extension() == Some(OsStr::new("rs")) {
                let mod_name_str = path.file_stem().unwrap().to_string_lossy();
                let mod_name = Ident::new(&mod_name_str, Span::call_site());
                modules.push(quote! {
                    pub mod #mod_name;
                });
            }
        }
    }

    Ok(quote! { #(#modules)* })
}

fn to_compile_error(msg: String) -> TokenStream {
    syn::Error::new(Span::call_site(), msg)
        .to_compile_error()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use std::fs;
    use tempfile::tempdir;

    fn assert_tokens_equal(expected: proc_macro2::TokenStream, actual: proc_macro2::TokenStream) {
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn test_include_folder_functionality() {
        let dir = tempdir().expect("Failed to create temp dir");
        let root = dir.path();

        fs::write(root.join("code.rs"), "").unwrap();
        let nested_dir = root.join("nested");
        fs::create_dir(&nested_dir).unwrap();
        fs::write(nested_dir.join("deep.rs"), "").unwrap();
        fs::write(root.join("mod.rs"), "// should be ignored").unwrap();

        let generated_tokens = generate_modules_recursive(root, &["mod.rs"]).unwrap();

        let expected_tokens = quote! {
            pub mod nested {
                pub mod deep;
            }
            pub mod code;
        };
        assert_tokens_equal(expected_tokens, generated_tokens);
    }

    #[test]
    fn test_include_all_modules_functionality() {
        let dir = tempdir().expect("Failed to create temp dir");
        let src_root = dir.path();

        fs::write(src_root.join("lib.rs"), "").unwrap(); // should be ignored
        fs::write(src_root.join("main.rs"), "").unwrap(); // should be ignored
        fs::write(src_root.join("mod.rs"), "").unwrap(); // should be included
        fs::write(src_root.join("api.rs"), "").unwrap(); // should be included

        let utils_dir = src_root.join("utils");
        fs::create_dir(&utils_dir).unwrap();
        fs::write(utils_dir.join("string_helpers.rs"), "").unwrap(); // should_be_included
        fs::write(utils_dir.join("lib.rs"), "").unwrap();

        let generated_tokens =
            generate_modules_recursive(src_root, &["lib.rs", "main.rs", "mod.rs"]).unwrap();

        let expected_tokens = quote! {
            pub mod utils {
                pub mod lib;
                pub mod string_helpers;
            }
            pub mod api;
        };
        assert_tokens_equal(expected_tokens, generated_tokens);
    }
}
