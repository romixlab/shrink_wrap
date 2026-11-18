use crate::ast::ty::Type;
use crate::codegen::util::CratePath;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{LitInt, LitStr};

/// Object size from `shrink_wrap` crate, copied here to decouple the two. Generated code refers to the `shrink_wrap` one.
/// Extensive description is in `shrink_wrap`.
#[derive(Clone, Debug)]
pub(crate) enum ObjectSize {
    Unsized,
    UnsizedFinalStructure,
    SelfDescribing,
    /// `size_bits` is the part known to the macro; `symbolic` holds terms it can't evaluate (an array length
    /// given as a const path, a `TailSize<N>` width), emitted as `size_bits + term + ..` for the compiler to fold.
    Sized {
        size_bits: usize,
        symbolic: Vec<TokenStream>,
    },
}

impl ObjectSize {
    pub(crate) fn sized(size_bits: usize) -> Self {
        Self::Sized {
            size_bits,
            symbolic: Vec::new(),
        }
    }

    pub(crate) fn sized_symbolic(term: TokenStream) -> Self {
        Self::Sized {
            size_bits: 0,
            symbolic: vec![term],
        }
    }

    /// `ElementSize::..` expression naming the `shrink_wrap` type through `cp`.
    pub(crate) fn tokens(&self, cp: &CratePath) -> TokenStream {
        let element_size = cp.item(quote! { ElementSize });
        match self {
            Self::Unsized => quote! { #element_size::Unsized },
            Self::UnsizedFinalStructure => quote! { #element_size::UnsizedFinalStructure },
            Self::SelfDescribing => quote! { #element_size::SelfDescribing },
            Self::Sized {
                size_bits,
                symbolic,
            } => {
                let size_bits = LitInt::new(format!("{size_bits}").as_str(), Span::call_site());
                quote! { #element_size::Sized { size_bits: #size_bits #(+ #symbolic)* } }
            }
        }
    }

    /// `ElementSize` const expression of `types` laid out one after another, on top of `self`. Sizes known to the
    /// macro are summed here, the rest (user types) is left to the compiler: `Sized { .. }.add(<T as ..>::ELEMENT_SIZE)`.
    pub(crate) fn sum_fields<'a>(
        &self,
        types: impl IntoIterator<Item = &'a Type>,
        is_ref: bool,
        cp: &CratePath,
    ) -> TokenStream {
        // NOTE: make sure to not accidentally bump Unsized to UFS here if any of the fields is UFS.
        // See ElementSize docs and comments on the add method.
        let mut sum = self.clone();
        let mut unknown = vec![];
        for ty in types {
            match ty.element_size() {
                Some(size) => sum = sum.add(size),
                None => unknown.push(ty.size_expr(is_ref, cp)),
            }
        }
        let sum = sum.tokens(cp);
        quote! { #sum #(.add(#unknown))* }
    }

    /// Size as a part of an error message: `8 bits`, `8 bits + N * 16`, `not Sized`.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Sized {
                size_bits,
                symbolic,
            } => {
                let mut s = format!("{size_bits} bit{}", if *size_bits == 1 { "" } else { "s" });
                for term in symbolic {
                    s += &format!(" + {term}");
                }
                s
            }
            _ => "not Sized".into(),
        }
    }

    /// Const assert that the type's `ELEMENT_SIZE` is what was asked for. `before` runs first in the same const
    /// block (a more specific check with a better message), `hint` is appended to the message.
    pub(crate) fn assert_element_size(
        &self,
        ident: &Ident,
        cfg: Option<&TokenStream>,
        is_ref: bool,
        before: TokenStream,
        hint: &str,
        cp: &CratePath,
    ) -> TokenStream {
        let element_size = cp.item(quote! { ElementSize });
        let size_ts = match self {
            Self::Unsized => quote! { #element_size::Unsized },
            Self::UnsizedFinalStructure => quote! { #element_size::UnsizedFinalStructure },
            Self::SelfDescribing => quote! { #element_size::SelfDescribing },
            Self::Sized { .. } => quote! { #element_size::Sized { .. } },
        };
        let size = match self {
            Self::Unsized => "Unsized",
            Self::UnsizedFinalStructure => "UnsizedFinalStructure",
            Self::SelfDescribing => "SelfDescribing",
            Self::Sized { .. } => "Sized",
        };
        let err_msg = format!("{ident} must be {size}{hint}");
        let err_msg = LitStr::new(&err_msg, Span::call_site());
        let cfg = cfg.map(|cond| quote! { #[cfg(#cond)] });
        let (ser_trait, des_trait) = cp.serdes_traits(is_ref);
        quote! {
            #cfg
            const _: () = {
                #before
                assert!(
                    matches!(<#ident as #ser_trait>::ELEMENT_SIZE, #size_ts),
                    #err_msg
                );
                assert!(
                    matches!(<#ident as #des_trait>::ELEMENT_SIZE, #size_ts),
                    #err_msg
                );
            };
        }
    }

    /// IMPORTANT: this method must be a copy of the one in `shrink_wrap`
    pub(crate) fn add(&self, other: Self) -> Self {
        // Order is very important here, size requirement is bumped from Sized to SelfDescribing to Unsized.
        // UFS is a bit tricky, it is "contagious", so that Vec<T> with T Unsized is UFS.
        // Structs and enums cannot accidentally become UFS, because by default they are Unsized, and no sum operations are
        // performed, otherwise it would have been a compatibility problem.
        match (self, other) {
            (Self::UnsizedFinalStructure, _) => Self::UnsizedFinalStructure,
            (_, Self::UnsizedFinalStructure) => Self::UnsizedFinalStructure,
            (Self::Unsized, _) => Self::Unsized,
            (_, Self::Unsized) => Self::Unsized,
            (Self::SelfDescribing, _) => Self::SelfDescribing,
            (_, Self::SelfDescribing) => Self::SelfDescribing,
            (
                Self::Sized {
                    size_bits: size_a,
                    symbolic: symbolic_a,
                },
                Self::Sized {
                    size_bits: size_b,
                    symbolic: symbolic_b,
                },
            ) => Self::Sized {
                size_bits: *size_a + size_b,
                symbolic: symbolic_a.iter().cloned().chain(symbolic_b).collect(),
            },
        }
    }
}
