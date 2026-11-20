use crate::ast::ty::Type;
use crate::codegen::util::CratePath;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{TokenStreamExt, quote};

#[derive(Clone)]
pub(crate) enum FieldPath {
    Ref(TokenStream),
    Value(TokenStream),
}

impl FieldPath {
    pub(crate) fn by_ref(self) -> TokenStream {
        match self {
            Self::Ref(path) => path,
            Self::Value(path) => quote! { &#path },
        }
    }

    pub(crate) fn by_value(self) -> TokenStream {
        match self {
            Self::Ref(path) => quote! { *#path },
            Self::Value(path) => path,
        }
    }

    fn into_inner(self) -> TokenStream {
        match self {
            Self::Ref(path) | Self::Value(path) => path,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Ref(path) | Self::Value(path) => path.is_empty(),
        }
    }
}

impl Type {
    pub(crate) fn def(&self, no_alloc: bool, cp: &CratePath) -> TokenStream {
        match self {
            Self::Bool | Self::IsSome(_) | Self::IsOk(_) => quote! { bool },
            Self::Nibble => cp.item(quote! { Nibble }),
            Self::U8 => quote! { u8 },
            Self::U16 => quote! { u16 },
            Self::UNib32 => cp.item(quote! { UNib32 }),
            Self::INib32 => cp.item(quote! { INib32 }),
            Self::IVlq32 => cp.item(quote! { IVlq32 }),
            Self::IVlq64 => cp.item(quote! { IVlq64 }),
            Self::UVlq32 => cp.item(quote! { UVlq32 }),
            Self::UVlq64 => cp.item(quote! { UVlq64 }),
            Self::U32 => quote! { u32 },
            Self::U64 => quote! { u64 },
            Self::U128 => quote! { u128 },
            Self::I8 => quote! { i8 },
            Self::I16 => quote! { i16 },
            Self::I32 => quote! { i32 },
            Self::I64 => quote! { i64 },
            Self::I128 => quote! { i128 },
            Self::F32 => quote! { f32 },
            Self::F64 => quote! { f64 },
            // Type::Bytes => {
            //     if no_alloc {
            //         quote! { RefVec<'i, u8> }
            //     } else {
            //         quote! { Vec<u8> }
            //     }
            // }
            Self::String => {
                if no_alloc {
                    quote! { &'i str }
                } else {
                    quote! { String }
                }
            }
            Self::Array(len, ty) => {
                let item_ty = ty.def(no_alloc, cp);
                quote! { [#item_ty; #len] }
            }
            Self::Tuple(types) => {
                let types = types.iter().map(|ty| ty.def(no_alloc, cp));
                quote! { ( #(#types),* ) }
            }
            Self::Vec(inner_ty) => {
                let inner_ty = inner_ty.def(no_alloc, cp);
                if no_alloc {
                    let ref_vec = cp.item(quote! { RefVec });
                    quote! { #ref_vec<'i, #inner_ty> }
                } else {
                    quote! { Vec<#inner_ty> }
                }
            }
            // Type::User(user_layout) => {
            //     let path = user_layout.path();
            //     quote! { #path }
            // }
            Self::External {
                path,
                has_lifetime: is_lifetime,
            } => path.tokens((*is_lifetime && no_alloc).then(|| quote! { 'i })),
            Self::Result(_, ok_err_ty) => {
                let ok_ty = ok_err_ty.0.def(no_alloc, cp);
                let err_ty = ok_err_ty.1.def(no_alloc, cp);
                quote! { Result<#ok_ty, #err_ty> }
            }
            Self::Option(_, option_ty) => {
                let option_ty = option_ty.def(no_alloc, cp);
                quote! { Option<#option_ty> }
            }
            Self::Range(ty) => {
                let ty = ty.def(no_alloc, cp);
                quote! { core::ops::Range<#ty> }
            }
            Self::RangeInclusive(ty) => {
                let ty = ty.def(no_alloc, cp);
                quote! { core::ops::RangeInclusive<#ty> }
            }
            Self::RefBox(box_ty) => {
                let box_ty = box_ty.def(no_alloc, cp);
                if no_alloc {
                    let ref_box = cp.item(quote! { RefBox });
                    quote! { #ref_box<'i, #box_ty> }
                } else {
                    quote! { Box<#box_ty>}
                }
            }
            Self::TailSize(width) => {
                let tail_size = cp.item(quote! { TailSize });
                if let Some(width) = width {
                    quote! { #tail_size<{ #width }> }
                } else {
                    quote! { #tail_size }
                }
            }
            Self::MaxLen(len, inner_ty) => {
                let max_len = cp.item(quote! { MaxLen });
                let inner_ty = inner_ty.def(no_alloc, cp);
                // a literal, a const or a `{ .. }` block as written, anything else needs the braces
                let len = match len {
                    syn::Expr::Lit(_) | syn::Expr::Path(_) | syn::Expr::Block(_) => quote! { #len },
                    len => quote! { { #len } },
                };
                quote! { #max_len<#inner_ty, #len> }
            }
            Self::Wire { plain, .. } => plain.def(no_alloc, cp),
        }
    }

    /// `<W as WireAs<T>>` of a `#[wire(W)] field: T` (DER-19).
    fn wire_as(plain: &Self, wire: &Self, cp: &CratePath) -> TokenStream {
        let wire_as = cp.item(quote! { WireAs });
        let plain = plain.def(true, cp);
        let wire = wire.def(true, cp);
        quote! { <#wire as #wire_as<#plain>> }
    }

    /// Width of a `TailSize` slot in bytes, as an expression.
    fn tail_size_width(width: &Option<syn::Expr>) -> TokenStream {
        if let Some(width) = width {
            quote! { { #width } }
        } else {
            quote! { 5 }
        }
    }

    pub(crate) fn buf_write(
        &self,
        field_path: FieldPath,
        no_alloc: bool,
        handle_eob: TokenStream,
        cp: &CratePath,
        tokens: &mut TokenStream,
    ) {
        let write_fn = match self {
            Self::Wire {
                plain, wire, user, ..
            } => {
                // range-checked: a value the wire type can't hold is an error, never cut
                let wire_as = Self::wire_as(plain, wire, cp);
                let value = if *user {
                    // a user type `T` isn't always `Copy`
                    let value = field_path.by_ref();
                    quote! { ::core::clone::Clone::clone(#value) }
                } else {
                    field_path.by_value()
                };
                let mut write = TokenStream::new();
                wire.buf_write(
                    FieldPath::Value(quote! { __wire }),
                    no_alloc,
                    handle_eob.clone(),
                    cp,
                    &mut write,
                );
                tokens.append_all(quote! {{
                    let __wire = #wire_as::from_plain(#value) #handle_eob;
                    #write
                }});
                return;
            }
            Self::Bool => "write_bool",
            Self::Nibble => "write_nib",
            Self::U8 => "write_u8",
            Self::U16 => "write_u16",
            Self::UNib32
            | Self::INib32
            | Self::UVlq32
            | Self::UVlq64
            | Self::IVlq32
            | Self::IVlq64
            | Self::Array(_, _)
            | Self::Tuple(_) => {
                let field_path = field_path.by_ref();
                tokens.append_all(quote! { wr.write(#field_path) #handle_eob; });
                return;
            }
            Self::U32 => "write_u32",
            Self::U64 => "write_u64",
            Self::U128 => "write_u128",
            Self::I8 => "write_i8",
            Self::I16 => "write_i16",
            Self::I32 => "write_i32",
            Self::I64 => "write_i64",
            Self::I128 => "write_i128",
            Self::F32 => "write_f32",
            Self::F64 => "write_f64",
            Self::IsSome(option_field) => {
                let path = if field_path.is_empty() {
                    quote! { #option_field }
                } else {
                    let path = field_path.by_value();
                    quote! { #path.#option_field }
                };
                tokens.append_all(quote! { wr.write_bool_latch(#path.is_some()); });
                return;
            }
            Self::Option(_, _ty) => {
                // handled explicitly, because is_some flag could be relocated using #[flag] attribute and generic
                // Option implementation cannot rely on it
                let field_path = field_path.by_ref();
                tokens.append_all(quote! {
                    if let Some(val) = #field_path {
                        wr.write(val) #handle_eob;
                    }
                });
                return;
            }
            Self::IsOk(result_field) => {
                let path = if field_path.is_empty() {
                    quote! { #result_field }
                } else {
                    let path = field_path.by_value();
                    quote! { #path.#result_field }
                };
                tokens.append_all(quote! { wr.write_bool_latch(#path.is_ok()); });
                return;
            }
            Self::Result(_flag_ident, _ok_err_ty) => {
                // handled explicitly, because is_ok flag could be relocated using #[flag] attribute and generic
                // Err implementation cannot rely on it
                let field_path = field_path.by_ref();
                tokens.append_all(quote! {
                    match #field_path {
                        Ok(val) => {
                            wr.write(val) #handle_eob;
                        }
                        Err(err) => {
                            wr.write(err) #handle_eob;
                        }
                    }
                });
                return;
            }
            Self::TailSize(width) => {
                // the field's value is ignored: the slot is filled in by `backfill_tail_size` (see `Type::backfill_tail_size`)
                // once the fields after it are written
                let width = Self::tail_size_width(width);
                tokens.append_all(
                    quote! { let __tail_size_slot = wr.reserve_tail_size(#width) #handle_eob; },
                );
                return;
            }
            Self::Vec(inner_ty) => {
                let is_vec_u8 = matches!(&**inner_ty, Self::U8);
                if is_vec_u8 && no_alloc {
                    let field_path = field_path.into_inner();
                    tokens
                        .append_all(quote! { #field_path.ser_shrink_wrap_vec_u8(wr) #handle_eob; });
                } else {
                    let field_path = field_path.by_ref();
                    tokens.append_all(quote! { wr.write(#field_path) #handle_eob; });
                }
                return;
            }
            Self::External { .. } => {
                let field_path = field_path.by_ref();
                // `write_field` is `write`, but plain numbers and `transparent` wrappers of them keep the error
                // for `wr.latched()` (`SerializeShrinkWrap::ser_shrink_wrap_latch`)
                tokens.append_all(quote! { wr.write_field(#field_path) #handle_eob; });
                return;
            }
            Self::String
            | Self::RefBox(_)
            | Self::Range(_)
            | Self::RangeInclusive(_)
            | Self::MaxLen(_, _) => {
                let field_path = field_path.by_ref();
                // same as Sized, special handling of Unsized moved to the BufWriter::write and BufRead::read instead
                tokens.append_all(quote! { wr.write(#field_path) #handle_eob; });
                return;
            }
        };
        let field_path = field_path.by_value();
        if self.is_latched() {
            // no check here: the error is kept in the writer until `wr.latched()` at the end of the type
            let write_fn = Ident::new(&format!("{write_fn}_latch"), Span::call_site());
            tokens.append_all(quote! { wr.#write_fn(#field_path); });
        } else {
            let write_fn = Ident::new(write_fn, Span::call_site());
            tokens.append_all(quote! { wr.#write_fn(#field_path) #handle_eob; });
        }
    }

    /// Plain types that `BufReader` / `BufWriter` have `_latch` methods for: read or written without an error
    /// check of their own, the type's code ends with one `latched()?` instead.
    pub(crate) fn is_latched(&self) -> bool {
        matches!(
            self,
            Self::Bool
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::F32
                | Self::IsSome(_)
                | Self::IsOk(_)
        )
    }

    /// Fill in the slot reserved by [`buf_write`](Self::buf_write) for a `TailSize` field, after all the fields
    /// of the struct or enum variant are written.
    pub(crate) fn backfill_tail_size(tokens: &mut TokenStream) {
        tokens.append_all(quote! { wr.backfill_tail_size(__tail_size_slot)?; });
    }

    pub(crate) fn buf_read(
        &self,
        variable_name: &Ident,
        owned: bool,
        handle_err: TokenStream,
        // false for a field with a default: its read must see the error to fall back
        latch: bool,
        cp: &CratePath,
        tokens: &mut TokenStream,
    ) {
        let read = if owned {
            quote! { read_owned }
        } else {
            quote! { read }
        };
        let read_fn = match self {
            Self::Wire { plain, wire, .. } => {
                // read as the wire type, then range-checked into the field's type: a wider or variable-length wire
                // type can hold more than the field
                let wire_as = Self::wire_as(plain, wire, cp);
                let wire = wire.def(true, cp);
                tokens.append_all(quote! {
                    let #variable_name = rd.#read::<#wire>().and_then(#wire_as::into_plain) #handle_err;
                });
                return;
            }
            Self::TailSize(_) => {
                // the slot bounds the rest of the value: a longer buffer stops at the value's end, a shorter one
                // is an error, and fields a newer writer added after the ones known here are skipped
                let ty = self.def(!owned, cp);
                tokens.append_all(quote! {
                    // the fields before the slot were read from this reader, the ones after it have their own
                    rd.latched()?;
                    let #variable_name: #ty = rd.#read()?;
                    let mut __tail_rd = rd.split(#variable_name.0 as usize)?;
                    let rd = &mut __tail_rd;
                });
                return;
            }
            Self::Bool | Self::IsOk(_) | Self::IsSome(_) => "read_bool",
            Self::Nibble => "read_nib",
            Self::U8 => "read_u8",
            Self::U16 => "read_u16",
            Self::U32 => "read_u32",
            Self::U64 => "read_u64",
            Self::U128 => "read_u128",
            Self::I8 => "read_i8",
            Self::I16 => "read_i16",
            Self::I32 => "read_i32",
            Self::I64 => "read_i64",
            Self::I128 => "read_i128",
            Self::F32 => "read_f32",
            Self::F64 => "read_f64",
            Self::External { .. } if latch => {
                // `read_field` is `read`, but `U1`..`U64`, `I2`..`I64`, plain numbers and `transparent` wrappers of
                // them keep the error for `rd.latched()?` (`DeserializeShrinkWrap::des_shrink_wrap_latch`)
                let read_field = if owned {
                    quote! { read_field_owned }
                } else {
                    quote! { read_field }
                };
                tokens.append_all(quote! {
                    let #variable_name = rd.#read_field() #handle_err;
                });
                return;
            }
            Self::UNib32
            | Self::INib32
            | Self::UVlq32
            | Self::UVlq64
            | Self::IVlq32
            | Self::IVlq64
            | Self::Array(_, _)
            | Self::Tuple(_)
            | Self::Vec(_)
            | Self::String
            | Self::RefBox(_)
            | Self::Range(_)
            | Self::RangeInclusive(_)
            | Self::MaxLen(_, _)
            | Self::External { .. } => {
                tokens.append_all(quote! {
                    let #variable_name = rd.#read() #handle_err;
                });
                return;
            }
            Self::Result(flag_ident, _ok_err_ty) => {
                let is_ok = &flag_ident;
                tokens.append_all(quote! {
                    let #variable_name = if #is_ok {
                        Ok(rd.#read() #handle_err)
                    } else {
                        Err(rd.#read() #handle_err)
                    };
                });
                return;
            }
            Self::Option(flag_ident, _option_ty) => {
                let is_some = &flag_ident;
                tokens.append_all(quote! {
                    let #variable_name = if #is_some {
                        Some(rd.#read() #handle_err)
                    } else {
                        None
                    };
                });
                return;
            }
        };
        if latch && self.is_latched() {
            // no check here: the error is kept in the reader until `rd.latched()?` at the end of the type
            let read_fn = Ident::new(&format!("{read_fn}_latch"), Span::call_site());
            tokens.append_all(quote! { let #variable_name: _ = rd.#read_fn(); });
            return;
        }
        let read_fn = Ident::new(read_fn, Span::call_site());
        tokens.append_all(quote! { let #variable_name: _ = rd.#read_fn() #handle_err; });
    }
}
