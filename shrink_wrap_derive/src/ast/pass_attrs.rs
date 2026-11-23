//! Attributes the derive doesn't read on a field or a variant, and which generated type each goes on (SW-86).

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::{Attribute, Meta, Token};

/// `#[attr_owned(..)]`: the attributes inside go on the owned type only.
const OWNED: &str = "attr_owned";
/// `#[attr_borrowed(..)]`: the attributes inside go on the borrowed type only.
const BORROWED: &str = "attr_borrowed";

/// The attributes the derive doesn't read on one field or variant, by the generated type they go on.
#[derive(Clone, Debug, Default)]
pub(crate) struct PassAttrs {
    /// As written: on the type with the item's own name, except `serde` ones (see [AttrSide::serde]).
    kept: Vec<Attribute>,
    /// The contents of `#[attr_owned(..)]`.
    owned: Vec<Attribute>,
    /// The contents of `#[attr_borrowed(..)]`.
    borrowed: Vec<Attribute>,
}

/// Which generated type the attributes are emitted for.
#[derive(Clone, Debug)]
pub(crate) struct AttrSide {
    /// The type with the item's own name.
    pub(crate) base: bool,
    /// The owned type, or an ambiguous type that is both.
    pub(crate) owned: bool,
    /// The borrowed type, or an ambiguous type that is both.
    pub(crate) borrowed: bool,
    pub(crate) serde: SerdeSide,
}

/// Where a `#[serde(..)]` (or a `cfg_attr` holding only `serde(..)`) goes.
#[derive(Clone, Debug)]
pub(crate) enum SerdeSide {
    /// No generated type visibly derives `Serialize` or `Deserialize`: with the other kept attributes.
    Kept,
    /// This type derives one of them: always (`None`), or under each `cfg_attr` condition that does.
    On(Option<Vec<TokenStream>>),
    /// Another generated type derives them, this one doesn't.
    Off,
}

impl AttrSide {
    /// A generated type that takes none of the attributes (the `discriminants` enum).
    pub(crate) fn none() -> Self {
        AttrSide {
            base: false,
            owned: false,
            borrowed: false,
            serde: SerdeSide::Off,
        }
    }
}

impl PassAttrs {
    /// Splits `attrs` into the kept ones and the contents of `attr_owned(..)` / `attr_borrowed(..)`.
    pub(crate) fn new(attrs: Vec<Attribute>) -> Result<Self, String> {
        let mut pass = PassAttrs::default();
        for attr in attrs {
            let side = if attr.path().is_ident(OWNED) {
                &mut pass.owned
            } else if attr.path().is_ident(BORROWED) {
                &mut pass.borrowed
            } else {
                pass.kept.push(attr);
                continue;
            };
            let name = attr.path().to_token_stream();
            let metas = attr
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|e| format!("#[{name}(attr, ..)]: {e}"))?;
            if metas.is_empty() {
                return Err(format!(
                    "#[{name}(..)] is empty, it takes the attributes to put there"
                ));
            }
            side.extend(metas.into_iter().map(|meta| Attribute {
                pound_token: attr.pound_token,
                style: attr.style,
                bracket_token: attr.bracket_token,
                meta,
            }));
        }
        Ok(pass)
    }

    /// The attributes for the generated type `side`.
    pub(crate) fn tokens(&self, side: &AttrSide) -> TokenStream {
        let mut ts = TokenStream::new();
        for attr in &self.kept {
            match (&side.serde, is_serde(attr)) {
                (SerdeSide::On(None), true) => attr.to_tokens(&mut ts),
                (SerdeSide::On(Some(conds)), true) => {
                    let meta = &attr.meta;
                    ts.extend(quote! { #(#[cfg_attr(#conds, #meta)])* });
                }
                (SerdeSide::Off, true) => {}
                (SerdeSide::Kept, _) | (_, false) => {
                    if side.base {
                        attr.to_tokens(&mut ts);
                    }
                }
            }
        }
        if side.owned {
            ts.extend(self.owned.iter().map(ToTokens::to_token_stream));
        }
        if side.borrowed {
            ts.extend(self.borrowed.iter().map(ToTokens::to_token_stream));
        }
        ts
    }
}

/// `#[serde(..)]`, or a `#[cfg_attr(pred, ..)]` holding `serde(..)` attributes only.
fn is_serde(attr: &Attribute) -> bool {
    if attr.path().is_ident("serde") {
        return true;
    }
    if !attr.path().is_ident("cfg_attr") {
        return false;
    }
    let Ok(metas) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return false;
    };
    metas.len() > 1 && metas.iter().skip(1).all(|m| m.path().is_ident("serde"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn side(base: bool, owned: bool, serde: SerdeSide) -> AttrSide {
        AttrSide {
            base,
            owned,
            borrowed: !owned,
            serde,
        }
    }

    fn pass(attrs: Vec<Attribute>) -> PassAttrs {
        PassAttrs::new(attrs).unwrap()
    }

    #[test]
    fn one_side_attributes() {
        let p = pass(vec![
            parse_quote!(#[attr_owned(serde(rename = "o"), allow(dead_code))]),
            parse_quote!(#[attr_borrowed(serde(rename = "b"))]),
        ]);
        let owned = p.tokens(&side(false, true, SerdeSide::Kept)).to_string();
        assert_eq!(
            owned,
            quote!(#[serde(rename = "o")] #[allow(dead_code)]).to_string()
        );
        let borrowed = p.tokens(&side(true, false, SerdeSide::Kept)).to_string();
        assert_eq!(borrowed, quote!(#[serde(rename = "b")]).to_string());
    }

    #[test]
    fn serde_follows_the_derive() {
        let p = pass(vec![
            parse_quote!(#[serde(with = "m")]),
            parse_quote!(#[cfg_attr(feature = "x", serde(rename = "r"))]),
            parse_quote!(#[allow(dead_code)]),
        ]);
        let base_off = p.tokens(&side(true, false, SerdeSide::Off)).to_string();
        assert_eq!(base_off, quote!(#[allow(dead_code)]).to_string());
        let owned_on = p
            .tokens(&side(
                false,
                true,
                SerdeSide::On(Some(vec![quote!(feature = "serde")])),
            ))
            .to_string();
        assert_eq!(
            owned_on,
            quote!(
                #[cfg_attr(feature = "serde", serde(with = "m"))]
                #[cfg_attr(feature = "serde", cfg_attr(feature = "x", serde(rename = "r")))]
            )
            .to_string()
        );
        let kept = p.tokens(&side(true, false, SerdeSide::Kept)).to_string();
        assert_eq!(
            kept,
            quote!(
                #[serde(with = "m")]
                #[cfg_attr(feature = "x", serde(rename = "r"))]
                #[allow(dead_code)]
            )
            .to_string()
        );
    }

    #[test]
    fn mixed_cfg_attr_is_not_serde() {
        assert!(!is_serde(
            &parse_quote!(#[cfg_attr(all(), serde(rename = "r"), allow(dead_code))])
        ));
        assert!(!is_serde(&parse_quote!(#[cfg_attr(all())])));
    }

    #[test]
    fn empty_is_refused() {
        let err = PassAttrs::new(vec![parse_quote!(#[attr_owned()])]).unwrap_err();
        assert!(err.contains("attr_owned"), "{err}");
    }
}
