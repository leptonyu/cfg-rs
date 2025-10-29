//! Auto derive FromConfig.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]
use quote::{__private::TokenStream, quote};
use syn::*;

#[allow(missing_docs)]
#[proc_macro_derive(FromConfig, attributes(config))]
pub fn derive_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let body = match input.data {
        Data::Struct(data) => derive_config_struct(&name, input.attrs, data),
        _ => panic!("Only support struct"),
    };
    proc_macro::TokenStream::from(quote! {#body})
}

fn derive_config_struct(name: &Ident, attrs: Vec<Attribute>, data: DataStruct) -> TokenStream {
    // Resolve cfg-rs crate path without relying on proc_macro_crate.
    // Default to ::cfg_rs, allow override via #[config(crate = "your_crate_name")]
    let mut cfg_crate_path = quote!(::cfg_rs);

    let fields = derive_config_fields(data);
    let fs: Vec<Ident> = fields.iter().map(|f| f.name.clone()).collect();
    let rs: Vec<&str> = fields.iter().map(|f| f.ren.as_str()).collect();
    let ds: Vec<TokenStream> = fields
        .iter()
        .map(|f| match &f.def {
            Some(d) => quote! {,Some(#d.into())},
            _ => quote! {,None},
        })
        .collect();
    let body = quote! {
        Self {
                #(#fs: context.parse_config(#rs #ds)?,)*
        }
    };

    let prefix = match derive_config_prefix(attrs, &mut cfg_crate_path) {
        Some(p) => quote! {
            #[automatically_derived]
            impl #cfg_crate_path::FromConfigWithPrefix for #name {
                fn prefix() -> &'static str {
                    #p
                }
            }
        },
        _ => quote! {},
    };

    quote! {
        #[automatically_derived]
        impl #cfg_crate_path::FromConfig for #name {
            fn from_config(
                context: &mut #cfg_crate_path::ConfigContext<'_>,
                value: ::core::option::Option<#cfg_crate_path::ConfigValue<'_>>,
            ) -> ::core::result::Result<Self, #cfg_crate_path::ConfigError> {
                ::core::result::Result::Ok(#body)
            }
        }

        #prefix
    }
}

fn derive_config_prefix(attrs: Vec<Attribute>, crate_path: &mut TokenStream) -> Option<String> {
    let mut prefix = None;
    for attr in attrs {
        if attr.path().is_ident("config") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("prefix") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    prefix = Some(s.value());
                    Ok(())
                } else if meta.path.is_ident("crate") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    let ident = Ident::new(&s.value(), s.span());
                    *crate_path = quote!(#ident);
                    Ok(())
                } else {
                    Err(meta.error("Only support prefix"))
                }
            })
            .unwrap();
        }
        if prefix.is_some() {
            break;
        }
    }
    prefix
}

struct FieldInfo {
    name: Ident,
    def: Option<String>,
    ren: String,
    desc: Option<String>,
}

fn derive_config_fields(data: DataStruct) -> Vec<FieldInfo> {
    if let Fields::Named(fields) = data.fields {
        let mut fs = vec![];
        for field in fields.named {
            fs.push(derive_config_field(field));
        }
        return fs;
    }
    panic!("Only support named body");
}

fn derive_config_field(field: Field) -> FieldInfo {
    let name = field.ident.expect("Not possible");
    let mut f = FieldInfo {
        ren: name.to_string(),
        name,
        def: None,
        desc: None,
    };
    derive_config_field_attr(&mut f, field.attrs);
    f
}

fn derive_config_field_attr(f: &mut FieldInfo, attrs: Vec<Attribute>) {
    for attr in attrs {
        if attr.path().is_ident("config") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    f.def = Some(parse_lit(meta.value()?.parse::<Lit>()?));
                } else if meta.path.is_ident("name") {
                    f.ren = parse_lit(meta.value()?.parse::<Lit>()?);
                } else if meta.path.is_ident("desc") {
                    f.desc = Some(parse_lit(meta.value()?.parse::<Lit>()?));
                } else {
                    return Err(meta.error("Only support default/name/desc"));
                }
                Ok(())
            })
            .unwrap();
        }
    }
}

fn parse_lit(lit: Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        Lit::ByteStr(s) => match String::from_utf8(s.value()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        },
        Lit::Byte(b) => (b.value() as char).to_string(),
        Lit::Int(i) => i.base10_digits().to_owned(),
        Lit::Float(f) => f.base10_digits().to_owned(),
        Lit::Bool(b) => b.value.to_string(),
        Lit::Char(c) => c.value().to_string(),
        Lit::Verbatim(_) => panic!("cfg-rs not support Verbatim"),
        _ => panic!("cfg-rs not support new types"),
    }
}
