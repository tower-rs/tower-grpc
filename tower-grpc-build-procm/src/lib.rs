extern crate proc_macro;
use proc_macro::TokenStream;
extern crate tower_grpc_build;
//use syn::parse::{Parse, ParseStream, Result};
//use syn::spanned::Spanned;
//use syn::{parse_macro_input, Expr, Ident, Token, Type, Visibility};
//
//
//struct ProtoParas {
//    protos_dir: Expr,
//    includes_dir: Expr,
//    service_name: Ident,
//    messages_names: Expr,
//}


#[proc_macro]
pub fn include_proto(input: TokenStream) -> TokenStream {
    // parameters order: protos dir, includes dir, service name, messages names
    let mut input_content = input.to_string();
    input_content.retain(|c| c != ' ');
    input_content.retain(|c| c != '\n');
    let paras: Vec<&str> = input_content.split(",").collect();
    println!("{:?}", paras);
    let msgs = paras[3].replace("|", ",");
    //println!("{:?}", msgs);
    let _output = tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&[paras[0]], &[paras[1]])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
    println!("{:?}", _output);

    let gen = format!("pub mod {} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\")); }} use {}::{{{}}};", paras[2], paras[2], paras[2], msgs);
    gen.parse().unwrap()
}