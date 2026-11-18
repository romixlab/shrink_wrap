use crate::ast::object_size::ObjectSize;
use crate::ast::path::Path;
use crate::codegen::util::CratePath;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Expr, LitInt};

/// Length of a `[T; N]` field: an integer literal the macro can do math with, or any other const expression
/// (`MAX_SERIES`, `N * 2`, `{ .. }`) emitted as written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ArrayLen {
    Lit(usize),
    Expr(Expr),
}

impl ToTokens for ArrayLen {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Lit(len) => {
                LitInt::new(format!("{len}").as_str(), Span::call_site()).to_tokens(tokens);
            }
            Self::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Bool,

    Nibble,
    U8,
    U16,
    U32,
    U64,
    U128,

    UNib32,
    UVlq32,
    UVlq64,

    I8,
    I16,
    I32,
    I64,
    I128,
    /// Zigzag over `UNib32` (SW-24): the signed counterpart, small negative and positive values both one nibble.
    INib32,
    /// Zigzag over `UVlq32` and `UVlq64` (SW-82): the signed byte-based numbers.
    IVlq32,
    IVlq64,

    F32,
    F64,

    // Bytes,
    String,

    Array(ArrayLen, Box<Self>),
    Tuple(Vec<Self>),
    Vec(Box<Self>),
    Range(Box<Self>),
    RangeInclusive(Box<Self>),

    /// A user type; `has_lifetime` when it is written with one (`Foo<'i>`)
    External {
        path: Path,
        has_lifetime: bool,
    },

    // is_some_flag, optional_ty
    Option(Ident, Box<Self>),
    // Only used for relocation of is_some flag in structs and enum struct variants.
    IsSome(Ident),

    // is_ok_flag, (ok_ty, err_ty)
    Result(Ident, Box<(Self, Self)>),
    // Only used for relocation of is_ok flag in structs and enum struct variants.
    IsOk(Ident),

    RefBox(Box<Self>),

    /// `TailSize<N>`, the size of the rest of the enclosing value, backfilled; `None` is the default width.
    TailSize(Option<Expr>),

    /// `MaxLen<T, N>`: a `Vec`/`RefVec` or string `T` with at most `N` elements, `N` kept as written.
    MaxLen(Expr, Box<Self>),

    /// `#[wire(W)] field: T` (DER-19): a plain integer `T` in Rust, written as the integer type `W` on the wire; or
    /// any `T` written as a user type `W` (`user`, DER-23), through `W: WireAs<T>`. `cfg` is the predicate of
    /// `#[cfg_attr(pred, wire(W))]`.
    Wire {
        plain: Box<Self>,
        wire: Box<Self>,
        cfg: Option<syn::Meta>,
        user: bool,
    },
}

impl Type {
    pub(crate) fn make_owned(&mut self) {
        match self {
            Self::External {
                path,
                has_lifetime: potential_lifetimes,
            } => {
                // Type::Unsized(path, potential_lifetimes) | Type::Sized(path, potential_lifetimes) => {
                if *potential_lifetimes {
                    path.make_owned();
                    *potential_lifetimes = false;
                }
            }
            Self::Option(_, some_ty) => some_ty.make_owned(),
            Self::Result(_, ok_err_ty) => {
                ok_err_ty.0.make_owned();
                ok_err_ty.1.make_owned();
            }
            Self::Array(_, layout) => {
                layout.make_owned();
            }
            Self::Tuple(types) => {
                for ty in types {
                    ty.make_owned();
                }
            }
            Self::Vec(layout) => {
                layout.make_owned();
            }
            Self::RefBox(ref_box) => {
                ref_box.make_owned();
            }
            Self::MaxLen(_, inner) => inner.make_owned(),
            _ => {}
        }
    }

    /// Whether `pred` holds for this type or any type nested in it.
    pub(crate) fn any(&self, pred: &dyn Fn(&Self) -> bool) -> bool {
        if pred(self) {
            return true;
        }
        match self {
            Self::Array(_, ty)
            | Self::Vec(ty)
            | Self::Range(ty)
            | Self::RangeInclusive(ty)
            | Self::Option(_, ty)
            | Self::RefBox(ty)
            | Self::MaxLen(_, ty) => ty.any(pred),
            Self::Tuple(types) => types.iter().any(|ty| ty.any(pred)),
            Self::Result(_, ok_err_ty) => ok_err_ty.0.any(pred) || ok_err_ty.1.any(pred),
            Self::Wire { wire, .. } => wire.any(pred),
            _ => false,
        }
    }

    /// `PhantomData<..>` (recognized by name) or `()`: zero bits on the wire, never anything else (DER-15). The other
    /// fields of a `transparent` struct (DER-18).
    pub(crate) fn is_zero_bits(&self) -> bool {
        match self {
            Self::Tuple(types) => types.is_empty(),
            Self::External { path, .. } => path
                .segments
                .last()
                .is_some_and(|ident| ident == "PhantomData"),
            _ => false,
        }
    }

    /// Whether this type is or contains `UVlq32Backfill`, recognized by name only.
    pub(crate) fn contains_backfill(&self) -> bool {
        self.contains_external(&["UVlq32Backfill"])
    }

    /// Whether this type is or contains `TailBytes` or `TailBytesOwned`, recognized by name only.
    pub(crate) fn contains_tail_bytes(&self) -> bool {
        self.contains_external(&["TailBytes", "TailBytesOwned"])
    }

    /// Whether this type is or contains `TailSize<N>`.
    pub(crate) fn contains_tail_size(&self) -> bool {
        self.any(&|ty| matches!(ty, Self::TailSize(_)))
    }

    /// Whether this type is or contains an external type whose last path segment is one of `names`.
    fn contains_external(&self, names: &[&str]) -> bool {
        self.any(&|ty| match ty {
            Self::External { path, .. } => path
                .segments
                .last()
                .is_some_and(|ident| names.iter().any(|name| ident == name)),
            _ => false,
        })
    }

    /// Every user type (`Type::External`) in this type, nested ones included, with whether it takes a lifetime.
    pub(crate) fn externals(&self, out: &mut Vec<(Path, bool)>) {
        match self {
            Self::External {
                path,
                has_lifetime: is_lifetime,
            } => out.push((path.clone(), *is_lifetime)),
            Self::Array(_, ty)
            | Self::Vec(ty)
            | Self::Range(ty)
            | Self::RangeInclusive(ty)
            | Self::Option(_, ty)
            | Self::RefBox(ty)
            | Self::MaxLen(_, ty) => ty.externals(out),
            Self::Tuple(types) => {
                for ty in types {
                    ty.externals(out);
                }
            }
            Self::Result(_, ok_err_ty) => {
                ok_err_ty.0.externals(out);
                ok_err_ty.1.externals(out);
            }
            Self::Wire { wire, .. } => wire.externals(out),
            _ => {}
        }
    }

    /// Const `bool` expression: whether writing a field of this type with `wr.write()` (or its `write_*`
    /// method) pushes a length to the reverse length FIFO. `None` when it never does, so that a type made of
    /// plain fields gets a literal `false`. What `SerializeShrinkWrap::PUSHES_LEN` of a derived type is made of.
    pub(crate) fn pushes_len_expr(&self, cp: &CratePath) -> Option<TokenStream> {
        match self {
            Self::String | Self::Vec(_) | Self::RefBox(_) | Self::MaxLen(_, _) => {
                Some(quote! { true })
            }
            Self::External {
                path,
                has_lifetime: is_lifetime,
            } => {
                let element_size = cp.item(quote! { ElementSize });
                let (ser_trait, _) = cp.serdes_traits(true);
                // 'static, because it is a const context
                let ty = path.tokens(is_lifetime.then(|| quote! { 'static }));
                // an Unsized value gets a size slot from `wr.write()`, the others push what they push
                Some(quote! {
                    (matches!(<#ty as #ser_trait>::ELEMENT_SIZE, #element_size::Unsized)
                        || <#ty as #ser_trait>::PUSHES_LEN)
                })
            }
            Self::Array(_, ty)
            | Self::Range(ty)
            | Self::RangeInclusive(ty)
            | Self::Option(_, ty) => ty.pushes_len_expr(cp),
            Self::Tuple(types) => Self::any_pushes_len(types.iter(), cp),
            Self::Wire { wire, .. } => wire.pushes_len_expr(cp),
            Self::Result(_, ok_err_ty) => {
                Self::any_pushes_len([&ok_err_ty.0, &ok_err_ty.1].into_iter(), cp)
            }
            // integers, flags, varints, the TailSize slot (its backfill encodes what was pushed after it)
            _ => None,
        }
    }

    /// Const `Option<MaxSize>` expression: upper bound of what a field of this type writes, see
    /// `SerializeShrinkWrap::MAX_SIZE`. Only for the borrowed side, the owned traits don't have it.
    pub(crate) fn max_size_expr(&self, cp: &CratePath) -> TokenStream {
        let max_size = cp.item(quote! { MaxSize });
        let some = |size: TokenStream| quote! { Some(#max_size::#size) };
        match self {
            Self::Bool | Self::IsSome(_) | Self::IsOk(_) => some(quote! { bits(1) }),
            Self::Nibble => some(quote! { nibbles(1) }),
            Self::U8 | Self::I8 => some(quote! { bytes(1) }),
            Self::U16 | Self::I16 => some(quote! { bytes(2) }),
            Self::U32 | Self::I32 | Self::F32 => some(quote! { bytes(4) }),
            Self::U64 | Self::I64 | Self::F64 => some(quote! { bytes(8) }),
            Self::U128 | Self::I128 => some(quote! { bytes(16) }),
            // unbounded
            Self::String | Self::Vec(_) | Self::RefBox(_) => quote! { None },
            // the flag is a field of its own (IsSome / IsOk), only the value is written here
            Self::Option(_, ty) => Self::write_max_size(ty, cp),
            // a user `W` may promise less for the plain values it takes (`WireAs::PLAIN_MAX_SIZE`)
            Self::Wire {
                plain,
                wire,
                user: true,
                ..
            } => {
                let size = cp.item(quote! { wire_as::plain_write_max_size });
                let (plain, wire) = (plain.def(true, cp), wire.def(true, cp));
                quote! { #size::<#wire, #plain>() }
            }
            Self::Wire { wire, .. } => wire.max_size_expr(cp),
            Self::Result(_, ok_err_ty) => {
                let ok = Self::write_max_size(&ok_err_ty.0, cp);
                let err = Self::write_max_size(&ok_err_ty.1, cp);
                quote! { #max_size::one_of(#ok, #err) }
            }
            // the slot, and when it is filled in, the lengths encoded after the fields: up to a padding nibble and
            // an alignment to a byte, which 2 more bytes cover
            Self::TailSize(width) => {
                let width = if let Some(width) = width {
                    quote! { (#width) }
                } else {
                    quote! { 5 }
                };
                some(quote! { bytes(#width + 2) })
            }
            // written with `wr.write()`: what the type's impl says
            Self::UNib32
            | Self::INib32
            | Self::IVlq32
            | Self::IVlq64
            | Self::UVlq32
            | Self::UVlq64
            | Self::Array(_, _)
            | Self::Tuple(_)
            | Self::Range(_)
            | Self::RangeInclusive(_)
            | Self::External { .. }
            | Self::MaxLen(_, _) => Self::write_max_size(self, cp),
        }
    }

    /// `MaxSize::of_write::<T>()` of this type, as a const expression.
    fn write_max_size(ty: &Self, cp: &CratePath) -> TokenStream {
        let max_size = cp.item(quote! { MaxSize });
        let ty = ty.def(true, cp);
        quote! { #max_size::of_write::<#ty>() }
    }

    /// `MaxSize::seq` of [`max_size_expr`](Self::max_size_expr) over `types`, fields one after another.
    pub(crate) fn seq_max_size<'a>(
        types: impl Iterator<Item = &'a Self>,
        cp: &CratePath,
    ) -> TokenStream {
        let max_size = cp.item(quote! { MaxSize });
        let sizes = types.map(|ty| ty.max_size_expr(cp));
        quote! {{
            let size = Some(#max_size::ZERO);
            #(let size = #max_size::seq(size, #sizes);)*
            size
        }}
    }

    /// `||` of [`pushes_len_expr`](Self::pushes_len_expr) over `types`, `None` when none of them pushes a length.
    pub(crate) fn any_pushes_len<'a>(
        types: impl Iterator<Item = &'a Self>,
        cp: &CratePath,
    ) -> Option<TokenStream> {
        let exprs: Vec<_> = types.filter_map(|ty| ty.pushes_len_expr(cp)).collect();
        if exprs.is_empty() {
            None
        } else {
            Some(quote! { (#(#exprs)||*) })
        }
    }

    /// Return `ElementSize` if it is known. None is returned for Unsized.
    pub(crate) fn element_size(&self) -> Option<ObjectSize> {
        let size_bits = match self {
            Self::Bool => 1,
            Self::Nibble => 4,
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::U64 => 64,
            Self::U128 => 128,
            Self::UNib32 => return Some(ObjectSize::SelfDescribing),
            Self::UVlq32 => return Some(ObjectSize::SelfDescribing),
            Self::UVlq64 => return Some(ObjectSize::SelfDescribing),
            Self::I8 => 8,
            Self::I16 => 16,
            Self::I32 => 32,
            Self::I64 => 64,
            Self::I128 => 128,
            Self::INib32 | Self::IVlq32 | Self::IVlq64 => {
                return Some(ObjectSize::SelfDescribing);
            }
            Self::F32 => 32,
            Self::F64 => 64,
            Self::String => return Some(ObjectSize::Unsized),
            Self::Array(len, ty) => {
                let size = match ty.element_size()? {
                    ObjectSize::Unsized => ObjectSize::Unsized,
                    ObjectSize::UnsizedFinalStructure => ObjectSize::UnsizedFinalStructure,
                    ObjectSize::SelfDescribing => ObjectSize::SelfDescribing,
                    ObjectSize::Sized {
                        size_bits,
                        symbolic,
                    } => match len {
                        ArrayLen::Lit(len) => ObjectSize::Sized {
                            size_bits: len * size_bits,
                            symbolic: symbolic
                                .iter()
                                .map(|term| quote! { #len * (#term) })
                                .collect(),
                        },
                        ArrayLen::Expr(len) => ObjectSize::sized_symbolic(
                            quote! { (#len) * (#size_bits #(+ #symbolic)*) },
                        ),
                    },
                };
                return Some(size);
            }
            Self::Tuple(types) => {
                let mut sum = ObjectSize::sized(0);
                for ty in types {
                    sum = sum.add(ty.element_size()?);
                }
                return Some(sum);
            }
            Self::Vec(_) => return Some(ObjectSize::UnsizedFinalStructure),
            Self::Range(ty) | Self::RangeInclusive(ty) => {
                // start and end
                let size = ty.element_size()?;
                return Some(size.add(size.clone()));
            }
            Self::External { .. } => return None, // cannot know if it's actually Unsized or not, const calculation will be performed instead
            Self::IsSome(_) | Self::IsOk(_) => return Some(ObjectSize::sized(1)),
            Self::Result(_, ok_err_ty) => {
                let mut sum = ObjectSize::SelfDescribing;
                sum = sum.add(ok_err_ty.0.element_size()?);
                sum = sum.add(ok_err_ty.1.element_size()?);
                return Some(sum);
            }
            Self::Option(_, option_ty) => {
                return Some(option_ty.element_size()?.add(ObjectSize::SelfDescribing));
            }
            Self::RefBox(_) => return Some(ObjectSize::Unsized),
            // same as the list or string inside
            Self::MaxLen(_, inner) => return inner.element_size(),
            Self::Wire { wire, .. } => return wire.element_size(),
            Self::TailSize(None) => 5 * 8,
            Self::TailSize(Some(width)) => {
                if let Expr::Lit(lit) = width
                    && let syn::Lit::Int(lit_int) = &lit.lit
                    && let Ok(width) = lit_int.base10_parse::<usize>()
                {
                    width * 8
                } else {
                    return Some(ObjectSize::sized_symbolic(quote! { 8 * (#width) }));
                }
            }
        };
        Some(ObjectSize::sized(size_bits))
    }

    /// `ElementSize` const expression of this type, also when a user type is somewhere inside
    /// ([`element_size`](Self::element_size) gives up on those): the compiler folds it.
    pub(crate) fn size_expr(&self, is_ref: bool, cp: &CratePath) -> TokenStream {
        if let Some(size) = self.element_size() {
            return size.tokens(cp);
        }
        let element_size = cp.item(quote! { ElementSize });
        match self {
            Self::External {
                path,
                has_lifetime: is_lifetime,
            } => {
                let (ser_trait, _) = cp.serdes_traits(is_ref);
                // 'static, because it is a const context
                let ty = path.tokens((*is_lifetime && is_ref).then(|| quote! { 'static }));
                quote! { <#ty as #ser_trait>::ELEMENT_SIZE }
            }
            Self::Array(len, ty) => {
                let size = ty.size_expr(is_ref, cp);
                quote! { #size.repeat(#len) }
            }
            Self::Tuple(types) => {
                let sizes = types.iter().map(|ty| ty.size_expr(is_ref, cp));
                quote! { #element_size::Sized { size_bits: 0 } #(.add(#sizes))* }
            }
            Self::Range(ty) | Self::RangeInclusive(ty) => {
                let size = ty.size_expr(is_ref, cp);
                quote! { #size.add(#size) }
            }
            Self::Option(_, ty) => {
                let size = ty.size_expr(is_ref, cp);
                quote! { #size.add(#element_size::SelfDescribing) }
            }
            Self::Wire { wire, .. } => wire.size_expr(is_ref, cp),
            Self::Result(_, ok_err_ty) => {
                let ok = ok_err_ty.0.size_expr(is_ref, cp);
                let err = ok_err_ty.1.size_expr(is_ref, cp);
                quote! { #element_size::SelfDescribing.add(#ok).add(#err) }
            }
            // element_size() knows every other type
            _ => ObjectSize::Unsized.tokens(cp),
        }
    }
}
