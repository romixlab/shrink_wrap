use crate::ast::docs::Docs;
use crate::ast::item_struct::Field;
use crate::ast::object_size::ObjectSize;
use crate::ast::path::Path;
use crate::ast::repr::Repr;
use crate::ast::ty::Type;
use crate::ast::util::{Cfg, Version};
use proc_macro2::Ident;
use syn::LitStr;

#[derive(Clone, Debug)]
pub struct ItemEnum {
    pub docs: Docs,
    pub derive: Vec<Path>,
    pub size_assumption: Option<ObjectSize>,
    pub repr: Repr,
    pub explicit_ww_repr: bool,
    pub ident: Ident,
    pub variants: Vec<Variant>,
    pub cfg: Option<Cfg>,
}

#[derive(Clone, Debug)]
pub enum Fields {
    Named(Vec<Field>),
    Unnamed(Vec<Type>),
    Unit,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub docs: Docs,
    pub ident: Ident,
    pub fields: Fields,
    pub discriminant: u32,
    pub since: Option<Version>,
}

impl ItemEnum {
    pub fn to_owned(&self, feature: LitStr) -> Self {
        let mut owned = self.clone();
        owned.ident = Ident::new(format!("{}Owned", self.ident).as_str(), self.ident.span());
        owned.cfg = Some(Cfg(feature));
        for v in &mut owned.variants {
            match &mut v.fields {
                Fields::Named(named) => {
                    for f in named {
                        f.ty.make_owned();
                    }
                }
                Fields::Unnamed(unnamed) => {
                    for f in unnamed {
                        f.make_owned();
                    }
                }
                Fields::Unit => {}
            }
        }
        owned
    }

    pub fn potential_lifetimes(&self) -> bool {
        for variant in &self.variants {
            if variant.potential_lifetimes() {
                return true;
            }
        }
        false
    }
}

impl Variant {
    pub fn potential_lifetimes(&self) -> bool {
        match &self.fields {
            Fields::Named(fields) => {
                for field in fields {
                    if field.ty.potential_lifetimes() {
                        return true;
                    }
                }
            }
            Fields::Unnamed(types) => {
                for ty in types {
                    if ty.potential_lifetimes() {
                        return true;
                    }
                }
            }
            Fields::Unit => {}
        }
        false
    }
}
