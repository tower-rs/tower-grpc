extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn include_proto(item: TokenStream) -> TokenStream {
    let item2 = item.to_string();
    let dirs: Vec<&str> = item2.split(",").collect();
    format!("tower_grpc_build::Config::new().enable_server(true)\
    .enable_client(true)\
    .build(&[{}], &[{}])\
    .unwrap_or_else(|e| panic!(\"protobuf compilation failed: {{}}\", e));", dirs[0], dirs[1]).parse().unwrap()
}