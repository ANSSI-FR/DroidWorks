// allowing panics since this is the standard way to show an
// error message from a proc-macro derive crate.
#![allow(clippy::panic)]

//! This crate introduces a proc macro derive to specifically derive
//! `dw_dex::instrs::Instruction` implementation for Dalvik instructions,
//! from proc macro attributes.
//!
//! Since there are many different instructions involved in Dalvik bytecode,
//! the goal is to have a clear definition of each opcode mnemonic, parsing
//! format (to derive instruction size in bytes), and other attributes that
//! can be relevant for subsequent analysis algorithms (for example, the
//! attribute `can_throw` to help during control flow graph construction).

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DeriveInput, Expr, ExprLit, Fields, Ident, Lit,
    LitBool, LitInt, LitStr, Meta, MetaNameValue, NestedMeta, Variant,
};

/// The main Dalvik bytecode `Instruction` proc macro derive.
///
/// It derives implementation of `dw_dex::instrs::Instruction` trait, using the
/// following attributes:
/// - `mnemonic` represent the mnemonic to be used when printing out bytecode instructions,
/// - `format` indicates the Dex format of the instruction for parsing
/// (see [Dalvik Executable instruction formats](https://source.android.com/devices/tech/dalvik/instruction-formats)),
/// - `can_throw` indicates if the instruction may throw an exception (default: `false`).
///
/// # Example
///
/// ```rust
/// trait Instruction {
///     fn mnemonic(&self) -> &str;
///     fn size(&self) -> usize;
///     fn can_throw(&self) -> bool;
/// }
///
/// #[derive(instruction_derive::Instruction)]
/// pub enum NopInstr {
///     // Waste cycles.
///     #[instruction(mnemonic = "nop", format = "10x")]
///     Nop,
/// }
/// ```
#[proc_macro_derive(Instruction, attributes(instruction))]
pub fn instruction_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let gen = derive_instruction_all(&ast);
    gen.into()
}

fn derive_instruction_all(ast: &DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let Data::Enum(data) = &ast.data else {
        panic!("#[derive(Instruction)] is only defined for enums")
    };

    let instruction_impl = derive_instruction_impl(name, data);

    quote! {
        #instruction_impl
    }
}

fn derive_instruction_impl(name: &Ident, data: &DataEnum) -> TokenStream2 {
    let mnemonic_matches = data
        .variants
        .iter()
        .map(|variant| mnemonic_match(name, variant))
        .collect::<Vec<TokenStream2>>();

    let size_matches = data
        .variants
        .iter()
        .map(|variant| size_match(name, variant))
        .collect::<Vec<TokenStream2>>();

    let canthrow_matches = data
        .variants
        .iter()
        .map(|variant| canthrow_match(name, variant))
        .collect::<Vec<TokenStream2>>();

    quote! {
        impl Instruction for #name {
            fn mnemonic(&self) -> &str {
                match self {
                    #(#mnemonic_matches)*
                }
            }

            fn size(&self) -> usize {
                match self {
                    #(#size_matches)*
                }
            }

            fn can_throw(&self) -> bool {
                match self {
                    #(#canthrow_matches)*
                }
            }
        }
    }
}

fn mnemonic_match(name: &Ident, variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    let fields = anonymous_fields_pattern(variant);
    let mnemonic = get_instruction_string_value(&variant.attrs, "mnemonic");

    quote! {
        #name::#ident #fields => #mnemonic,
    }
}

fn size_match(name: &Ident, variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    let fields = named_fields_pattern(variant);
    let format = get_instruction_string_value(&variant.attrs, "format").value();
    let size: Expr = if &format == "custom" {
        let size_attr = get_instruction_string_value(&variant.attrs, "size");
        size_attr.parse().expect("size")
    } else if !format.is_empty() && format.chars().next().expect("next char").is_ascii_digit() {
        let sz = &format[0..1];
        Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new(sz, Span::call_site())),
        })
    } else {
        panic!("bad 'format' attribute");
    };

    quote! {
        #name::#ident #fields => #size,
    }
}

fn canthrow_match(name: &Ident, variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    let fields = anonymous_fields_pattern(variant);
    let canthrow = get_instruction_bool_value(&variant.attrs, "can_throw");

    quote! {
        #name::#ident #fields => #canthrow,
    }
}

fn anonymous_fields_pattern(variant: &Variant) -> TokenStream2 {
    match &variant.fields {
        Fields::Named(_) => quote! { { .. } },
        Fields::Unnamed(flds) => {
            let voids: Vec<TokenStream2> = flds.unnamed.iter().map(|_| quote! { _ }).collect();
            quote! {(#(#voids),*)}
        }
        Fields::Unit => quote! {},
    }
}

fn named_fields_pattern(variant: &Variant) -> TokenStream2 {
    match &variant.fields {
        Fields::Named(flds) => {
            let params: Vec<_> = flds
                .named
                .iter()
                .map(|n| n.ident.clone().expect("identifier"))
                .collect();
            quote! {{ #(#params),* }}
        }
        Fields::Unnamed(flds) => {
            let params: Vec<_> = flds
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| Ident::new(&format!("_{i}"), Span::call_site()))
                .collect();
            quote! {(#(#params),*)}
        }
        Fields::Unit => quote! {},
    }
}

fn get_instruction_values(attr: &Attribute) -> Vec<MetaNameValue> {
    if !attr.path.is_ident("instruction") {
        return Vec::new();
    }

    match attr.parse_meta() {
        Ok(Meta::NameValue(v)) => vec![v],
        Ok(Meta::List(meta)) => meta
            .nested
            .into_iter()
            .map(|nested| match nested {
                NestedMeta::Meta(Meta::Path(path)) => {
                    let span = path
                        .segments
                        .first()
                        .expect("path first segment")
                        .ident
                        .span();
                    MetaNameValue {
                        path,
                        eq_token: syn::token::Eq { spans: [span] },
                        lit: Lit::Bool(LitBool { value: true, span }),
                    }
                }
                NestedMeta::Meta(Meta::NameValue(n)) => n,
                _ => panic!("expected #[instruction(...)]"),
            })
            .collect(),
        _ => panic!("expected #[instruction(...)]"),
    }
}

fn get_instruction_string_value(attrs: &[Attribute], name: &str) -> LitStr {
    for name_value in attrs.iter().flat_map(get_instruction_values) {
        if name_value.path.is_ident(name) {
            match &name_value.lit {
                Lit::Str(s) => return s.clone(),
                _ => panic!("expected string for '{name}' value"),
            }
        }
    }
    panic!("missing '{name}' attribute");
}

fn get_instruction_bool_value(attrs: &[Attribute], name: &str) -> LitBool {
    for name_value in attrs.iter().flat_map(get_instruction_values) {
        if name_value.path.is_ident(name) {
            match &name_value.lit {
                Lit::Bool(b) => return b.clone(),
                _ => panic!("expected bool for '{name}' value"),
            }
        }
    }
    LitBool {
        value: false,
        span: Span::call_site(),
    }
}
