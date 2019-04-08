#![allow(dead_code)]
#![allow(unused_variables)]
#![recursion_limit = "128"]
//#![feature(proc_macro_diagnostic)]
//#[cfg(feature = proc_macro_diagnostic)]
#[cfg_attr(feature = "nightly", proc_macro_diagnostic)]

extern crate proc_macro;
use proc_macro::TokenStream;
extern crate tower_grpc_build;

use quote::{quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, Lit, token, Token, braced, parenthesized};


struct ProtoParas {
    brace_token: token::Brace,
    para_pairs: Punctuated<ParaPair, Token![,]>,
}

struct ParaPair {
    paren_token: token::Paren,
    proto_file_name: Lit,
    inner_comma: Token![,],
    proto_mod_name: Ident,
}

impl Parse for ProtoParas {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(ProtoParas {
            brace_token: braced!(content in input),
            para_pairs: content.parse_terminated(ParaPair::parse)?,
        })
    }
}

impl Parse for ParaPair {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(ParaPair {
            paren_token: parenthesized!(content in input),
            proto_file_name: content.parse()?,
            inner_comma: content.parse()?,
            proto_mod_name: content.parse()?,
        })
    }
}

//TODO: add more syntax check
fn syntax_check<T>(input: &T){
    #[cfg(feature = "nightly")]
    (if input.to_string().is_empty() {
        input.span()
            .unwrap()
            .warning("please assign the name of file/module you want to include.")
            .emit();
    })
}

#[proc_macro]
pub fn include_proto(input: TokenStream) -> TokenStream {
    let ProtoParas {
        brace_token: _,
        para_pairs,
    } = parse_macro_input!(input as ProtoParas);

    let mut expanded = quote! {};
    for para_pair in para_pairs {
        let mod_name: Ident = para_pair.proto_mod_name;
        let file_name: Lit = para_pair.proto_file_name;

        if cfg!(feature = "nightly"){
            syntax_check(&mod_name);
            syntax_check(&file_name);
        }

        expanded = quote! {
            #expanded
            pub mod #mod_name {
                include!(concat!(env!("OUT_DIR"), #file_name));
            }
        };
    }


    TokenStream::from(expanded)
}