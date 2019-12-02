use std::convert::TryInto;

use failure::Error;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote_spanned;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

use super::Schema;
use crate::util::{JSONObject, JSONValue, SimpleIdent};

/// `parse_macro_input!` expects a TokenStream_1
struct AttrArgs {
    _paren_token: syn::token::Paren,
    args: Punctuated<syn::NestedMeta, Token![,]>,
}

impl Parse for AttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _paren_token: syn::parenthesized!(content in input),
            args: Punctuated::parse_terminated(&content)?,
        })
    }
}

/// Enums, provided they're simple enums, simply get an enum string schema attached to them.
pub fn handle_enum(
    mut attribs: JSONObject,
    mut enum_ty: syn::ItemEnum,
) -> Result<TokenStream, Error> {
    if !attribs.contains_key("type") {
        attribs.insert(
            SimpleIdent::new("type".to_string(), Span::call_site()),
            JSONValue::new_ident(Ident::new("String", enum_ty.enum_token.span)),
        );
    }

    if let Some(fmt) = attribs.get("format") {
        bail!(fmt.span(), "illegal key 'format', will be autogenerated");
    }

    let schema = {
        let schema: Schema = attribs.try_into()?;
        let mut ts = TokenStream::new();
        schema.to_typed_schema(&mut ts)?;
        ts
    };

    // with_capacity(enum_ty.variants.len());
    // doesn't exist O.o
    let mut variants = Punctuated::<syn::LitStr, Token![,]>::new();
    for variant in &mut enum_ty.variants {
        match &variant.fields {
            syn::Fields::Unit => (),
            _ => bail!(variant => "api macro does not support enums with fields"),
        }

        let mut renamed = false;
        for attrib in &mut variant.attrs {
            if !attrib.path.is_ident("serde") {
                continue;
            }

            let args: AttrArgs = syn::parse2(attrib.tokens.clone())?;
            for arg in args.args {
                match arg {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(var)) => {
                        if var.path.is_ident("rename") {
                            match var.lit {
                                syn::Lit::Str(lit) => variants.push(lit),
                                _ => bail!(var.lit => "'rename' value must be a string literal"),
                            }
                            renamed = true;
                        }
                    }
                    _ => (), // ignore
                }
            }
        }

        if !renamed {
            let name = &variant.ident;
            variants.push(syn::LitStr::new(&name.to_string(), name.span()));
        }
    }

    let name = &enum_ty.ident;

    Ok(quote_spanned! { name.span() =>
        #enum_ty
        impl #name {
            pub const API_SCHEMA: &'static ::proxmox::api::schema::Schema =
                & #schema
                .format(&::proxmox::api::schema::ApiStringFormat::Enum(&[#variants]))
                .schema();
        }
    })
}