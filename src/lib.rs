use proc_macro2::{Ident, Span};
use quote::quote;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn include_folder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut root_path = PathBuf::from(manifest_dir);
    root_path.push(&path_str);

    // get module_name in path
    let module_name = Path::new(&path_str)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(".", "_"))
        .expect("Bad file name");

    // check root_path is a dir
    if !root_path.is_dir() {
        let error_msg = format!("Path '{}' is not a valid dir", root_path.display());
        return syn::Error::new_spanned(path_lit, error_msg)
            .to_compile_error()
            .into();
    }

    // Scan modules
    let modules = match generate_modules_for_dir(&root_path) {
        Ok(tokens) => tokens,
        Err(e) => return e.into(),
    };

    // Build top sentense
    let top_module_ident = Ident::new(&module_name, Span::call_site());
    let expanded = quote! {
        pub mod #top_module_ident {
            #modules
        }
    };

    expanded.into()
}

fn generate_modules_for_dir(
    dir: &Path,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let mut modules = Vec::new();

    for entry in fs::read_dir(dir).map_err(|e| to_compile_error(e.to_string()))? {
        let entry = entry.map_err(|e| to_compile_error(e.to_string()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if path.is_dir() {
            let mod_name = Ident::new(&file_name_str, Span::call_site());
            let inner_mods = generate_modules_for_dir(&path)?;

            if !inner_mods.is_empty() {
                modules.push(quote! {
                    pub mod #mod_name {
                        #inner_mods
                    }
                });
            }
        } else if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "rs" && file_name_str != "mod.rs" {
                    let mod_name_str = path.file_stem().unwrap().to_string_lossy();
                    let mod_name = Ident::new(&mod_name_str, Span::call_site());
                    modules.push(quote! {
                        pub mod #mod_name;
                    });
                }
            }
        }
    }

    Ok(quote! { #(#modules)* })
}

fn to_compile_error(msg: String) -> proc_macro2::TokenStream {
    syn::Error::new(Span::call_site(), msg).into_compile_error()
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
    fn test_generate_modules_for_dir_simple() {
        let dir = tempdir().expect("Failed to create temp dir");
        let root = dir.path();

        // ./code.rs
        // ./mod.rs (should be ignored)
        fs::write(root.join("code.rs"), "pub fn a() {}").unwrap();
        fs::write(root.join("mod.rs"), "// mod.rs").unwrap();

        let generated_tokens = generate_modules_for_dir(root).unwrap();

        let expected_tokens = quote! {
            pub mod code;
        };

        assert_tokens_equal(expected_tokens, generated_tokens);
    }

    #[test]
    fn test_generate_modules_with_nesting() {
        let dir = tempdir().expect("Failed to create temp dir");
        let root = dir.path();

        // ./code.rs
        // ./nested/
        // ./nested/deep.rs
        fs::write(root.join("code.rs"), "").unwrap();
        let nested_dir = root.join("nested");
        fs::create_dir(&nested_dir).unwrap();
        fs::write(nested_dir.join("deep.rs"), "").unwrap();

        let generated_tokens = generate_modules_for_dir(root).unwrap();

        let expected_tokens = quote! {
            pub mod nested {
                pub mod deep;
            }
            pub mod code;
        };

        assert_tokens_equal(expected_tokens, generated_tokens);
    }

    #[test]
    fn test_empty_directory() {
        let dir = tempdir().expect("Failed to create temp dir");
        let root = dir.path();

        fs::write(root.join("mod.rs"), "").unwrap();

        let generated_tokens = generate_modules_for_dir(root).unwrap();

        let expected_tokens = quote! {}; // should be empty token stream

        assert_tokens_equal(expected_tokens, generated_tokens);
    }

    #[test]
    fn test_directory_not_found() {
        let path = PathBuf::from("./non_existent_directory_for_test");
        let result = generate_modules_for_dir(&path);

        assert!(result.is_err());
    }
}
