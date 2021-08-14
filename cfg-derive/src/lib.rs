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

/// Auto derive config.
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
                #(#fs: context.parse_config(#rs#ds)?,)*
        }
    };

    let prefix = match derive_config_attr(attrs) {
        Some(p) => quote! {
            impl FromConfigWithPrefix for #name {
                fn prefix() -> &'static str {
                    #p
                }
            }
        },
        _ => quote! {},
    };

    quote! {
        impl FromConfig for #name {
            fn from_config(
                context: &mut ConfigContext<'_>,
                value: Option<ConfigValue<'_>>,
            ) -> Result<Self, ConfigError> {
                Ok(#body)
            }
        }

        #prefix
    }
}

fn derive_config_attr(attrs: Vec<Attribute>) -> Option<String> {
    for attr in attrs {
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_config(&list) {
                continue;
            }
            for m in list.nested {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = m {
                    if parse_path(nv.path) == "prefix" {
                        match nv.lit {
                            Lit::Str(s) => return Some(s.value()),
                            _ => panic!("Only support string"),
                        }
                    } else {
                        panic!("Only support prefix");
                    }
                } else {
                    panic!("Only support prefix=\"xxx\"");
                }
            }
        }
    }
    None
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
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_config(&list) {
                continue;
            }
            for m in list.nested {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = m {
                    match &parse_path(nv.path)[..] {
                        "default" => f.def = Some(parse_lit(nv.lit)),
                        "name" => f.ren = parse_lit(nv.lit),
                        "desc" => f.desc = Some(parse_lit(nv.lit)),
                        _ => panic!("Only support default/name/desc"),
                    }
                } else {
                    panic!("Only support NestedMeta::Meta(Meta::NameValue)");
                }
            }
        }
    }
}

fn is_config(list: &MetaList) -> bool {
    if let Some(v) = list.path.segments.iter().next() {
        return v.ident == "config";
    }
    false
}

fn parse_path(path: Path) -> String {
    path.segments.first().unwrap().ident.to_string()
}

fn parse_lit(lit: Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        Lit::ByteStr(s) => match String::from_utf8(s.value()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        },
        Lit::Int(i) => i.base10_digits().to_owned(),
        Lit::Float(f) => f.base10_digits().to_owned(),
        Lit::Bool(b) => b.value.to_string(),
        Lit::Char(c) => c.value().to_string(),
        Lit::Byte(b) => (b.value() as char).to_string(),
        Lit::Verbatim(_) => panic!("cfg-rs not support Verbatim"),
    }
}
