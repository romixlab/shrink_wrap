use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::LitStr;

#[derive(Clone, Debug)]
pub struct Docs {
    docs: Vec<LitStr>,
}

impl Docs {
    pub fn empty() -> Docs {
        Docs { docs: Vec::new() }
    }

    pub fn push(&mut self, s: LitStr) {
        self.docs.push(s);
    }

    pub fn push_str(&mut self, s: impl AsRef<str>) {
        self.docs.push(LitStr::new(s.as_ref(), Span::call_site()));
    }
}

impl ToTokens for Docs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for doc in &self.docs {
            tokens.extend(quote!(#[doc = #doc]));
        }
    }
}
