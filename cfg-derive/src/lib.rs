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
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::*;

#[allow(missing_docs)]
#[proc_macro_derive(FromConfig, attributes(config, validate))]
pub fn derive_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();
    let body = match input.data {
        Data::Struct(data) => derive_config_struct(&name, input.attrs, data),
        _ => Err(Error::new_spanned(name, "Only support struct")),
    };
    let body = match body {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    };
    proc_macro::TokenStream::from(body)
}

fn derive_config_struct(
    name: &Ident,
    attrs: Vec<Attribute>,
    data: DataStruct,
) -> Result<TokenStream> {
    // Resolve cfg-rs crate path without relying on proc_macro_crate.
    // Default to ::cfg_rs, allow override via #[config(crate = "your_crate_name")]
    let mut cfg_crate_path = quote!(::cfg_rs);

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

    let fields = derive_config_fields(data)?;
    let fs: Vec<Ident> = fields.iter().map(|f| f.name.clone()).collect();
    #[cfg(feature = "regex")]
    let regex_map = {
        use quote::ToTokens;
        let mut map: BTreeMap<String, Ident> = BTreeMap::new();
        let mut idx = 0usize;
        for field in &fields {
            for rule in &field.validates {
                if let ValidateRule::Regex { pattern, .. } = rule {
                    let key = pattern.to_token_stream().to_string();
                    if !map.contains_key(&key) {
                        let ident = quote::format_ident!("__CFG_REGEX_{}", idx);
                        idx += 1;
                        map.insert(key, ident);
                    }
                }
            }
        }
        map
    };
    #[cfg(not(feature = "regex"))]
    let regex_map: BTreeMap<String, Ident> = BTreeMap::new();

    let regex_cache_decl = if regex_map.is_empty() {
        quote! {}
    } else {
        let decls = regex_map.values().map(|ident| {
            quote! {
                static #ident: ::std::sync::OnceLock<
                    ::core::result::Result<::regex::Regex, ::std::string::String>
                > = ::std::sync::OnceLock::new();
            }
        });
        quote! { #(#decls)* }
    };

    let parse_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| build_parse_and_validate(f, &cfg_crate_path, &regex_map))
        .collect();

    Ok(quote! {
        #[automatically_derived]
        impl #cfg_crate_path::FromConfig for #name {
            fn from_config(
                context: &mut #cfg_crate_path::ConfigContext<'_>,
                value: ::core::option::Option<#cfg_crate_path::ConfigValue<'_>>,
            ) -> ::core::result::Result<Self, #cfg_crate_path::ConfigError> {
                #regex_cache_decl
                #(#parse_fields)*
                ::core::result::Result::Ok(Self {
                    #(#fs,)*
                })
            }
        }

        #prefix
    })
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
    ty: Type,
    validates: Vec<ValidateRule>,
}

fn derive_config_fields(data: DataStruct) -> Result<Vec<FieldInfo>> {
    if let Fields::Named(fields) = data.fields {
        let mut fs = vec![];
        for field in fields.named {
            fs.push(derive_config_field(field)?);
        }
        return Ok(fs);
    }
    Err(Error::new_spanned(data.fields, "Only support named body"))
}

fn derive_config_field(field: Field) -> Result<FieldInfo> {
    let name = field.ident.expect("Not possible");
    let mut f = FieldInfo {
        ren: name.to_string(),
        name,
        def: None,
        desc: None,
        ty: field.ty.clone(),
        validates: vec![],
    };
    derive_config_field_attr(&mut f, field.attrs)?;
    Ok(f)
}

fn derive_config_field_attr(f: &mut FieldInfo, attrs: Vec<Attribute>) -> Result<()> {
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
            })?;
        } else if attr.path().is_ident("validate") {
            parse_validate_attr(f, attr)?;
        }
    }
    Ok(())
}

enum ValidateRule {
    Range {
        min: Option<Expr>,
        max: Option<Expr>,
        message: Option<LitStr>,
    },
    NotEmpty {
        message: Option<LitStr>,
    },
    Length {
        min: Option<Expr>,
        max: Option<Expr>,
        message: Option<LitStr>,
    },
    #[cfg(feature = "regex")]
    Regex {
        pattern: Expr,
        message: Option<LitStr>,
    },
    Custom {
        path: Path,
        message: Option<LitStr>,
    },
}

fn parse_validate_attr(f: &mut FieldInfo, attr: Attribute) -> Result<()> {
    let mut rules: Vec<ValidateRule> = Vec::new();
    let mut message_seen = false;
    attr.parse_nested_meta(|meta| {
        let item = meta.path.get_ident().map(|i| i.to_string());
        let ret = match item.as_deref() {
            Some("range") => {
                let mut min: Option<Expr> = None;
                let mut max: Option<Expr> = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("min") {
                        let value = inner.value()?;
                        min = Some(value.parse::<Expr>()?);
                        Ok(())
                    } else if inner.path.is_ident("max") {
                        let value = inner.value()?;
                        max = Some(value.parse::<Expr>()?);
                        Ok(())
                    } else {
                        Err(inner.error("Only support min/max"))
                    }
                })?;
                rules.push(ValidateRule::Range {
                    min,
                    max,
                    message: None,
                });
                Ok(())
            }
            Some("length") => {
                let mut min: Option<Expr> = None;
                let mut max: Option<Expr> = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("min") {
                        let value = inner.value()?;
                        min = Some(value.parse::<Expr>()?);
                        Ok(())
                    } else if inner.path.is_ident("max") {
                        let value = inner.value()?;
                        max = Some(value.parse::<Expr>()?);
                        Ok(())
                    } else {
                        Err(inner.error("Only support min/max"))
                    }
                })?;
                rules.push(ValidateRule::Length {
                    min,
                    max,
                    message: None,
                });
                Ok(())
            }
            Some("not_empty") => {
                rules.push(ValidateRule::NotEmpty { message: None });
                Ok(())
            }
            #[cfg(feature = "regex")]
            Some("regex") => {
                let value = meta.value()?;
                let s: Expr = value.parse()?;
                rules.push(ValidateRule::Regex {
                    pattern: s,
                    message: None,
                });
                Ok(())
            }
            Some("custom") => {
                let value = meta.value()?;
                let path = if let Ok(p) = value.parse::<Path>() {
                    p
                } else {
                    let s: LitStr = value.parse()?;
                    parse_str::<Path>(&s.value()).map_err(|err| {
                        Error::new(
                            s.span(),
                            format!("custom validator must be a valid path: {}", err),
                        )
                    })?
                };
                rules.push(ValidateRule::Custom {
                    path,
                    message: None,
                });
                Ok(())
            }
            Some("message") => {
                let message = if let Ok(value) = meta.value() {
                    value.parse::<LitStr>()?
                } else {
                    meta.input.parse::<LitStr>()?
                };
                if rules.is_empty() {
                    return Err(meta.error("validate message must follow a rule"));
                }
                if message_seen {
                    return Err(meta.error("Only one message allowed per validate attribute"));
                }
                message_seen = true;
                if let Some(last) = rules.pop() {
                    if validate_rule_has_message(&last) {
                        return Err(meta.error("validate message already set for this rule"));
                    }
                    rules.push(apply_validate_message(last, Some(message)));
                }
                Ok(())
            }
            _ => Err(meta.error("Only support range/length/not_empty/regex/custom/message")),
        };

        ret
    })?;

    if rules.is_empty() {
        return Err(Error::new(
            attr.span(),
            "validate attribute must contain a rule",
        ));
    }
    f.validates.extend(rules);
    Ok(())
}

fn validate_rule_has_message(rule: &ValidateRule) -> bool {
    match rule {
        ValidateRule::Range { message, .. }
        | ValidateRule::NotEmpty { message }
        | ValidateRule::Length { message, .. }
        | ValidateRule::Custom { message, .. } => message.is_some(),
        #[cfg(feature = "regex")]
        ValidateRule::Regex { message, .. } => message.is_some(),
    }
}

fn apply_validate_message(rule: ValidateRule, message: Option<LitStr>) -> ValidateRule {
    match rule {
        ValidateRule::Range { min, max, .. } => ValidateRule::Range { min, max, message },
        ValidateRule::NotEmpty { .. } => ValidateRule::NotEmpty { message },
        ValidateRule::Length { min, max, .. } => ValidateRule::Length { min, max, message },
        #[cfg(feature = "regex")]
        ValidateRule::Regex { pattern, .. } => ValidateRule::Regex { pattern, message },
        ValidateRule::Custom { path, .. } => ValidateRule::Custom { path, message },
    }
}

fn build_parse_and_validate(
    field: &FieldInfo,
    crate_path: &TokenStream,
    regex_map: &BTreeMap<String, Ident>,
) -> TokenStream {
    let name = &field.name;
    let ty = &field.ty;
    let key = field.ren.as_str();
    let def = match &field.def {
        Some(d) => quote! {,Some(#d.into())},
        None => quote! {,None},
    };
    let validate = build_validate_block(field, crate_path, regex_map);
    if field.validates.is_empty() {
        quote! {
            let #name: #ty = context.parse_config(#key #def)?;
        }
    } else {
        quote! {
            let #name: #ty = context.parse_config(#key #def)?;
            #validate
        }
    }
}

fn build_validate_block(
    field: &FieldInfo,
    crate_path: &TokenStream,
    regex_map: &BTreeMap<String, Ident>,
) -> TokenStream {
    if field.validates.is_empty() {
        return quote! {};
    }

    let name = &field.name;
    let key = field.ren.as_str();
    let is_option = option_inner(&field.ty).is_some();

    let field_key_init = quote! {
        let field_key = || {
            let current_key = context.current_key();
            if current_key.is_empty() {
                #key.to_string()
            } else {
                format!("{}.{}", current_key, #key)
            }
        };
    };
    let field_key_expr = quote! { &field_key };

    if is_option {
        let value_expr = quote! { value };
        let checks: Vec<TokenStream> = field
            .validates
            .iter()
            .map(|rule| {
                build_validate_rule(rule, crate_path, &field_key_expr, &value_expr, regex_map)
            })
            .collect();
        quote! {
            #field_key_init
            if let ::core::option::Option::Some(value) = #name.as_ref() {
                #(#checks)*
            }
        }
    } else {
        let value_expr = quote! { &#name };
        let checks: Vec<TokenStream> = field
            .validates
            .iter()
            .map(|rule| {
                build_validate_rule(rule, crate_path, &field_key_expr, &value_expr, regex_map)
            })
            .collect();
        quote! {
            #field_key_init
            #(#checks)*
        }
    }
}

fn build_validate_rule(
    rule: &ValidateRule,
    crate_path: &TokenStream,
    field_key: &TokenStream,
    value: &TokenStream,
    _regex_map: &BTreeMap<String, Ident>,
) -> TokenStream {
    match rule {
        ValidateRule::Range { min, max, message } => {
            let min_expr = min
                .as_ref()
                .map(|v| quote! { ::core::option::Option::Some(&#v) });
            let max_expr = max
                .as_ref()
                .map(|v| quote! { ::core::option::Option::Some(&#v) });
            let min_ref = min_expr.unwrap_or_else(|| quote! { ::core::option::Option::None });
            let max_ref = max_expr.unwrap_or_else(|| quote! { ::core::option::Option::None });
            let call = quote! {
                #crate_path::validate::validate_range(
                    #field_key,
                    #value,
                    #min_ref,
                    #max_ref,
                )
            };
            wrap_validate_call(call, crate_path, field_key, message)
        }
        ValidateRule::Length { min, max, message } => {
            let min_expr = min
                .as_ref()
                .map(|v| quote! { ::core::option::Option::Some(#v) });
            let max_expr = max
                .as_ref()
                .map(|v| quote! { ::core::option::Option::Some(#v) });
            let min_ref = min_expr.unwrap_or_else(|| quote! { ::core::option::Option::None });
            let max_ref = max_expr.unwrap_or_else(|| quote! { ::core::option::Option::None });
            let call = quote! {
                #crate_path::validate::validate_length(
                    #field_key,
                    #value,
                    #min_ref,
                    #max_ref,
                )
            };
            wrap_validate_call(call, crate_path, field_key, message)
        }
        ValidateRule::NotEmpty { message } => {
            let call = quote! {
                #crate_path::validate::validate_not_empty(
                    #field_key,
                    #value,
                )
            };
            wrap_validate_call(call, crate_path, field_key, message)
        }
        #[cfg(feature = "regex")]
        ValidateRule::Regex { pattern, message } => {
            use quote::ToTokens;
            let key = pattern.to_token_stream().to_string();
            let regex_ident = _regex_map.get(&key).expect("missing regex cache entry");
            let call = quote! {
                {
                    let regex_result = #regex_ident.get_or_init(|| {
                        ::regex::Regex::new(#pattern)
                            .map_err(|err| format!("invalid regex: {}", err))
                    });

                    let regex = match regex_result {
                        ::core::result::Result::Ok(re) => re,
                        ::core::result::Result::Err(message) => {
                            return ::core::result::Result::Err(
                                #crate_path::ConfigError::ConfigParseError(
                                    (#field_key)(),
                                    message.clone(),
                                ),
                            );
                        }
                    };

                    #crate_path::validate::validate_regex(
                        #field_key,
                        regex,
                        #value.as_ref(),
                    )
                }
            };
            wrap_validate_call(call, crate_path, field_key, message)
        }
        ValidateRule::Custom { path, message } => {
            let call = quote! { #crate_path::validate::validate_custom(#field_key, #value, #path) };
            wrap_validate_call(call, crate_path, field_key, message)
        }
    }
}

fn wrap_validate_call(
    call: TokenStream,
    crate_path: &TokenStream,
    field_key: &TokenStream,
    message: &Option<LitStr>,
) -> TokenStream {
    if let Some(message) = message {
        quote! {
            match #call {
                ::core::result::Result::Ok(()) => (),
                ::core::result::Result::Err(_) => {
                    return ::core::result::Result::Err(
                        #crate_path::ConfigError::ConfigParseError(
                            (#field_key)(),
                            #message.to_string(),
                        ),
                    );
                }
            }
        }
    } else {
        quote! {
            #call?;
        }
    }
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    for arg in &args.args {
        if let GenericArgument::Type(inner) = arg {
            return Some(inner);
        }
    }
    None
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
