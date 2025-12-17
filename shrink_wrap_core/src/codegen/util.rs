use crate::ast::path::Path;
use crate::ast::util::Cfg;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};

pub(crate) fn serdes_scaffold(
    ty_name: &Ident,
    ser: impl ToTokens,
    des: impl ToTokens,
    lifetime: TokenStream,
    cfg: &Option<Cfg>,
    element_size: TokenStream,
) -> TokenStream {
    quote! {
        #cfg
        impl #lifetime SerializeShrinkWrap for #ty_name #lifetime {
            const ELEMENT_SIZE: ElementSize = #element_size;

            fn ser_shrink_wrap(&self, wr: &mut BufWriter) -> Result<(), ShrinkWrapError> {
                #ser
            }
        }

        #cfg
        impl<'i> DeserializeShrinkWrap<'i> for #ty_name #lifetime {
            const ELEMENT_SIZE: ElementSize = #element_size;

            fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, ShrinkWrapError> {
                #des
            }
        }
    }
}

pub(crate) fn strings_to_derive(traits: &Vec<Path>) -> TokenStream {
    if traits.is_empty() {
        quote! {}
    } else {
        // let traits = traits
        //     .iter()
        //     .map(|s| Ident::new(s.as_str(), Span::call_site()));
        quote! {
            #[derive(#(#traits),*)]
        }
    }
}
