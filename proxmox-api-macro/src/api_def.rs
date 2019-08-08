use std::convert::TryFrom;

use proc_macro2::{Ident, TokenStream};

use derive_builder::Builder;
use failure::{bail, Error};
use quote::quote_spanned;

use super::parsing::{Expression, Object};

#[derive(Clone)]
pub enum CliMode {
    Disabled,
    ParseCli, // By default we try proxmox::cli::ParseCli
    FromStr,
    Function(syn::Expr),
}

impl Default for CliMode {
    fn default() -> Self {
        CliMode::ParseCli
    }
}

impl TryFrom<Expression> for CliMode {
    type Error = Error;
    fn try_from(expr: Expression) -> Result<Self, Error> {
        if expr.is_ident("FromStr") {
            return Ok(CliMode::FromStr);
        }

        if let Ok(value) = expr.is_lit_bool() {
            return Ok(if value.value {
                CliMode::ParseCli
            } else {
                CliMode::Disabled
            });
        }

        Ok(CliMode::Function(expr.expect_expr()?))
    }
}

impl CliMode {
    pub fn quote(&self, name: &Ident) -> TokenStream {
        match self {
            CliMode::Disabled => quote_spanned! { name.span() => None },
            CliMode::ParseCli => quote_spanned! { name.span() =>
                Some(<#name as ::proxmox::api::cli::ParseCli>::parse_cli)
            },
            CliMode::FromStr => quote_spanned! { name.span() =>
                Some(<#name as ::proxmox::api::cli::ParseCliFromStr>::parse_cli)
            },
            CliMode::Function(func) => quote_spanned! { name.span() => Some(#func) },
        }
    }
}

#[derive(Builder)]
pub struct CommonTypeDefinition {
    pub description: syn::LitStr,
    #[builder(default)]
    pub cli: CliMode,
}

impl CommonTypeDefinition {
    fn builder() -> CommonTypeDefinitionBuilder {
        CommonTypeDefinitionBuilder::default()
    }

    pub fn from_object(obj: &mut Object) -> Result<Self, Error> {
        let mut def = Self::builder();

        if let Some(value) = obj.remove("description") {
            def.description(value.expect_lit_str()?);
        }
        if let Some(value) = obj.remove("cli") {
            def.cli(CliMode::try_from(value)?);
        }

        match def.build() {
            Ok(r) => Ok(r),
            Err(err) => bail!("{}", err),
        }
    }
}

#[derive(Builder)]
pub struct ParameterDefinition {
    #[builder(default)]
    pub default: Option<syn::Expr>,
    #[builder(default)]
    pub description: Option<syn::LitStr>,
    #[builder(default)]
    pub maximum: Option<syn::Expr>,
    #[builder(default)]
    pub minimum: Option<syn::Expr>,
    #[builder(default)]
    pub maximum_length: Option<syn::Expr>,
    #[builder(default)]
    pub minimum_length: Option<syn::Expr>,
    #[builder(default)]
    pub validate: Option<syn::Expr>,

    /// Formats are module paths. The module must contain a verify function:
    /// `fn verify(Option<&str>) -> bool`, and a `NAME` constant used in error messages to refer to
    /// the format name.
    #[builder(default)]
    pub format: Option<syn::Path>,

    /// Patterns are regular expressions. When a literal string is provided, a `lazy_static` regex
    /// is created for the verifier. Otherwise it is taken as an expression (i.e. a path) to an
    /// existing regex variable/method.
    #[builder(default)]
    pub pattern: Option<syn::Expr>,

    #[builder(default)]
    pub serialize_with: Option<syn::Path>,
    #[builder(default)]
    pub deserialize_with: Option<syn::Path>,
}

impl ParameterDefinition {
    pub fn builder() -> ParameterDefinitionBuilder {
        Default::default()
    }

    pub fn from_object(obj: Object) -> Result<Self, Error> {
        let mut def = ParameterDefinition::builder();

        let obj_span = obj.span();
        for (key, value) in obj {
            match key.as_str() {
                "default" => {
                    def.default(Some(value.expect_expr()?));
                }
                "description" => {
                    def.description(Some(value.expect_lit_str()?));
                }
                "maximum" => {
                    def.maximum(Some(value.expect_expr()?));
                }
                "minimum" => {
                    def.minimum(Some(value.expect_expr()?));
                }
                "maximum_length" => {
                    def.maximum_length(Some(value.expect_expr()?));
                }
                "minimum_length" => {
                    def.minimum_length(Some(value.expect_expr()?));
                }
                "validate" => {
                    def.validate(Some(value.expect_expr()?));
                }
                "format" => {
                    def.format(Some(value.expect_path()?));
                }
                "pattern" => {
                    def.pattern(Some(value.expect_expr()?));
                }
                "serialize_with" => {
                    def.serialize_with(Some(value.expect_path()?));
                }
                "deserialize_with" => {
                    def.deserialize_with(Some(value.expect_path()?));
                }
                "serialization" => {
                    let mut de = value.expect_path()?;
                    let mut ser = de.clone();
                    ser.segments.push(syn::PathSegment {
                        ident: Ident::new("serialize", obj_span),
                        arguments: syn::PathArguments::None,
                    });
                    de.segments.push(syn::PathSegment {
                        ident: Ident::new("deserialize", obj_span),
                        arguments: syn::PathArguments::None,
                    });
                    def.deserialize_with(Some(de));
                    def.serialize_with(Some(ser));
                }
                other => c_bail!(key.span(), "invalid key in type definition: {}", other),
            }
        }

        match def.build() {
            Ok(r) => Ok(r),
            Err(err) => c_bail!(obj_span, "{}", err),
        }
    }

    pub fn from_expression(expr: Expression) -> Result<Self, Error> {
        let span = expr.span();
        match expr {
            Expression::Expr(syn::Expr::Lit(lit)) => match lit.lit {
                syn::Lit::Str(description) => Ok(ParameterDefinition::builder()
                    .description(Some(description))
                    .build()
                    .map_err(|e| c_format_err!(span, "{}", e))?),
                _ => c_bail!(span, "expected description or field definition"),
            },
            Expression::Object(obj) => ParameterDefinition::from_object(obj),
            _ => c_bail!(span, "expected description or field definition"),
        }
    }
}
